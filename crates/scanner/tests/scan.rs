//! Behavior of the `scanner` public interface on real temporary folders.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use scanner::{scan, NodeId, NodeKind, Progress, ScanError, ScanOptions, ScanResult, ScanState};
use tempfile::TempDir;

fn write_file(path: &Path, size: u64) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    File::create(path).unwrap().set_len(size).unwrap();
}

fn tree(files: &[(&str, u64)]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, size) in files {
        write_file(&dir.path().join(path), *size);
    }
    dir
}

fn run(target: &Path) -> ScanResult {
    run_with(target, &ScanOptions::default())
}

fn run_with(target: &Path, options: &ScanOptions) -> ScanResult {
    scan(target, options, &|_| {}, &AtomicBool::new(false)).unwrap()
}

fn all_children(result: &ScanResult, id: NodeId) -> Vec<NodeId> {
    result.children(id, 0, usize::MAX)
}

fn child_named(result: &ScanResult, id: NodeId, name: &str) -> NodeId {
    all_children(result, id)
        .into_iter()
        .find(|&c| result.node(c).unwrap().name() == name)
        .unwrap_or_else(|| panic!("no child named {name}"))
}

fn child_names(result: &ScanResult, id: NodeId) -> Vec<String> {
    all_children(result, id)
        .into_iter()
        .map(|c| result.node(c).unwrap().name().to_string())
        .collect()
}

/// A comparable dump of the whole tree, in display order.
fn dump(result: &ScanResult) -> Vec<(PathBuf, u64, u64, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![result.root()];
    while let Some(id) = stack.pop() {
        let node = result.node(id).unwrap();
        out.push((
            result.path(id).unwrap(),
            node.size(),
            node.files(),
            node.folders(),
        ));
        let mut children = all_children(result, id);
        children.reverse();
        stack.extend(children);
    }
    out
}

#[test]
fn aggregates_sizes_and_counts_per_folder_and_total() {
    let dir = tree(&[
        ("a.bin", 100),
        ("docs/b.txt", 30),
        ("docs/c.txt", 20),
        ("docs/deep/d.txt", 5),
        ("media/e.mp4", 1000),
    ]);
    fs::create_dir(dir.path().join("empty")).unwrap();
    let result = run(dir.path());

    assert_eq!(result.state(), ScanState::Completed);
    assert_eq!(result.total_size(), 1155);
    assert_eq!(result.total_files(), 5);
    assert_eq!(result.total_folders(), 4);

    let root = result.root();
    let docs = result.node(child_named(&result, root, "docs")).unwrap();
    assert_eq!(docs.kind(), NodeKind::Folder);
    assert_eq!(docs.size(), 55);
    assert_eq!(docs.files(), 3);
    assert_eq!(docs.folders(), 1);

    let file = result.node(child_named(&result, root, "a.bin")).unwrap();
    assert_eq!(file.kind(), NodeKind::File);
    assert_eq!(file.size(), 100);
    assert_eq!((file.files(), file.folders()), (0, 0));
    assert!(file.modified().is_some());
}

#[test]
fn children_mix_files_and_folders_by_descending_size_then_name() {
    let dir = tree(&[
        ("small.txt", 1),
        ("big/x.bin", 500),
        ("mid.bin", 200),
        ("tie-b.bin", 50),
        ("tie-a.bin", 50),
    ]);
    let result = run(dir.path());
    assert_eq!(
        child_names(&result, result.root()),
        ["big", "mid.bin", "tie-a.bin", "tie-b.bin", "small.txt"]
    );
}

#[test]
fn children_are_pageable() {
    let dir = tree(&[("a", 5), ("b", 4), ("c", 3), ("d", 2), ("e", 1)]);
    let result = run(dir.path());
    let root = result.root();
    assert_eq!(result.node(root).unwrap().child_count(), 5);

    let page = |offset, limit| -> Vec<String> {
        result
            .children(root, offset, limit)
            .into_iter()
            .map(|id| result.node(id).unwrap().name().to_string())
            .collect()
    };
    assert_eq!(page(0, 2), ["a", "b"]);
    assert_eq!(page(2, 2), ["c", "d"]);
    assert_eq!(page(4, 2), ["e"]);
    assert!(page(9, 2).is_empty());
}

#[test]
fn paths_are_rebuilt_from_the_target() {
    let dir = tree(&[("one/two/three.txt", 1)]);
    let result = run(dir.path());
    let one = child_named(&result, result.root(), "one");
    let two = child_named(&result, one, "two");
    let three = child_named(&result, two, "three.txt");
    assert_eq!(result.path(result.root()).unwrap(), dir.path());
    assert_eq!(
        result.path(three).unwrap(),
        dir.path().join("one").join("two").join("three.txt")
    );
}

#[test]
fn empty_target() {
    let dir = tempfile::tempdir().unwrap();
    let result = run(dir.path());
    assert_eq!(result.state(), ScanState::Completed);
    assert_eq!(result.total_size(), 0);
    assert_eq!(result.total_files(), 0);
    assert_eq!(result.total_folders(), 0);
    assert!(all_children(&result, result.root()).is_empty());
    assert!(result.top_files(100).is_empty());
}

#[test]
fn zero_byte_files_are_counted() {
    let dir = tree(&[("empty1", 0), ("sub/empty2", 0)]);
    let result = run(dir.path());
    assert_eq!(result.total_size(), 0);
    assert_eq!(result.total_files(), 2);
    assert_eq!(result.total_folders(), 1);
}

#[test]
fn names_with_accents_spaces_and_emoji() {
    let names = [
        "naïve résumé.pdf",
        "café/crème brûlée 🎵.mp3",
        "日本語 フォルダ/ファイル.txt",
    ];
    let dir = tree(&[(names[0], 10), (names[1], 20), (names[2], 30)]);
    let result = run(dir.path());
    assert_eq!(result.total_size(), 60);
    assert_eq!(result.total_files(), 3);

    let cafe = child_named(&result, result.root(), "café");
    let song = child_named(&result, cafe, "crème brûlée 🎵.mp3");
    assert_eq!(result.node(song).unwrap().size(), 20);
    assert_eq!(result.path(song).unwrap(), dir.path().join(names[1]));
}

#[test]
fn missing_target_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let error = scan(
        &missing,
        &ScanOptions::default(),
        &|_| {},
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(error, ScanError::TargetNotFound(path) if path == missing));
}

#[test]
fn file_target_is_rejected() {
    let dir = tree(&[("file.txt", 1)]);
    let file = dir.path().join("file.txt");
    let error = scan(
        &file,
        &ScanOptions::default(),
        &|_| {},
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(error, ScanError::NotAFolder(_)));
}

#[test]
fn top_files_across_the_whole_target() {
    let dir = tree(&[
        ("a/huge.iso", 9000),
        ("a/b/c/big.zip", 5000),
        ("tiny.txt", 1),
        ("mid.mp4", 3000),
        ("d/small.doc", 10),
    ]);
    let options = ScanOptions {
        top_n: 3,
        ..ScanOptions::default()
    };
    let result = run_with(dir.path(), &options);
    let top: Vec<_> = result
        .top_files(100)
        .iter()
        .map(|&id| (result.path(id).unwrap(), result.node(id).unwrap().size()))
        .collect();
    assert_eq!(
        top,
        [
            (dir.path().join("a").join("huge.iso"), 9000),
            (dir.path().join("a/b/c/big.zip"), 5000),
            (dir.path().join("mid.mp4"), 3000),
        ]
    );
    assert_eq!(result.top_files(2).len(), 2);
}

#[test]
fn top_files_ties_are_deterministic() {
    let files: Vec<(String, u64)> = (0..20)
        .map(|i| (format!("dir{}/f{i}.bin", i % 3), 7))
        .collect();
    let files: Vec<(&str, u64)> = files.iter().map(|(p, s)| (p.as_str(), *s)).collect();
    let dir = tree(&files);
    let options = ScanOptions {
        top_n: 5,
        ..ScanOptions::default()
    };
    let paths = |result: &ScanResult| -> Vec<PathBuf> {
        result
            .top_files(5)
            .iter()
            .map(|&id| result.path(id).unwrap())
            .collect()
    };
    let first = paths(&run_with(dir.path(), &options));
    assert_eq!(first.len(), 5);
    for threads in [1, 2, 8] {
        let options = ScanOptions {
            threads: Some(threads),
            ..options.clone()
        };
        assert_eq!(paths(&run_with(dir.path(), &options)), first);
    }
}

#[test]
fn top_files_larger_than_file_count_returns_all_files() {
    let dir = tree(&[("a", 3), ("b/c", 2), ("b/d", 1)]);
    let result = run(dir.path());
    let sizes: Vec<u64> = result
        .top_files(100)
        .iter()
        .map(|&id| result.node(id).unwrap().size())
        .collect();
    assert_eq!(sizes, [3, 2, 1]);
}

#[test]
fn progress_is_reported_and_ends_at_the_total() {
    let files: Vec<(String, u64)> = (0..200).map(|i| (format!("d{}/f{i}", i % 10), i)).collect();
    let files: Vec<(&str, u64)> = files.iter().map(|(p, s)| (p.as_str(), *s)).collect();
    let dir = tree(&files);

    let events: Mutex<Vec<Progress>> = Mutex::new(Vec::new());
    let result = scan(
        dir.path(),
        &ScanOptions::default(),
        &|p| events.lock().unwrap().push(p.clone()),
        &AtomicBool::new(false),
    )
    .unwrap();

    let events = events.into_inner().unwrap();
    let last = events.last().expect("at least one progress event");
    assert_eq!(last.bytes, result.total_size());
    assert_eq!(last.files, result.total_files());
    assert!(events.windows(2).all(|w| w[0].bytes <= w[1].bytes));
}

#[test]
fn cancelling_returns_quickly_in_the_cancelled_state() {
    let files: Vec<(String, u64)> = (0..50).map(|i| (format!("d{}/f{i}", i % 5), 1)).collect();
    let files: Vec<(&str, u64)> = files.iter().map(|(p, s)| (p.as_str(), *s)).collect();
    let dir = tree(&files);

    let cancel = AtomicBool::new(true);
    let result = scan(dir.path(), &ScanOptions::default(), &|_| {}, &cancel).unwrap();
    assert_eq!(result.state(), ScanState::Cancelled);
    assert!(result.duration() < Duration::from_secs(2));
    assert!(result.total_files() < 50);
}

#[test]
fn cancelling_mid_scan_stops_before_opening_queued_folders() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.bin", "b.bin", "c.bin"] {
        write_file(&dir.path().join(name), 10);
    }
    for i in 0..400 {
        write_file(&dir.path().join(format!("d{}/e{}/f{i}", i % 20, i % 7)), 1);
    }

    // The first progress event comes right after the Target is listed and
    // before any subfolder is opened: cancelling there must leave all 20
    // queued subfolders unopened.
    let cancel = AtomicBool::new(false);
    let result = scan(
        dir.path(),
        &ScanOptions::default(),
        &|_| cancel.store(true, Ordering::Relaxed),
        &cancel,
    )
    .unwrap();

    assert_eq!(result.state(), ScanState::Cancelled);
    assert_eq!(result.total_files(), 3);
    assert_eq!(result.total_size(), 30);
    assert_eq!(result.total_folders(), 20);
    for id in all_children(&result, result.root()) {
        assert_eq!(result.node(id).unwrap().child_count(), 0);
    }
}

#[test]
fn parallel_and_sequential_results_are_identical() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..300u64 {
        write_file(
            &dir.path()
                .join(format!("a{}/b{}/c{}/f{i}.bin", i % 4, i % 9, i % 5)),
            (i * 37) % 101,
        );
    }
    let sequential = run_with(
        dir.path(),
        &ScanOptions {
            threads: Some(1),
            ..ScanOptions::default()
        },
    );
    let parallel = run_with(
        dir.path(),
        &ScanOptions {
            threads: Some(8),
            ..ScanOptions::default()
        },
    );
    assert_eq!(dump(&sequential), dump(&parallel));
    let top = |r: &ScanResult| -> Vec<PathBuf> {
        r.top_files(100)
            .iter()
            .map(|&id| r.path(id).unwrap())
            .collect()
    };
    assert_eq!(top(&sequential), top(&parallel));
}

#[test]
fn files_under_paths_longer_than_260_characters_are_counted() {
    let dir = tempfile::tempdir().unwrap();
    let segment = "a".repeat(50);
    let mut deep = dir.path().to_path_buf();
    for _ in 0..7 {
        deep.push(&segment);
    }
    let file = deep.join("deep-file.bin");
    assert!(file.as_os_str().len() > 260);
    write_file(&file, 4242);

    let result = run(dir.path());
    assert_eq!(result.total_size(), 4242);
    assert_eq!(result.total_files(), 1);
    assert_eq!(result.total_folders(), 7);
    let id = result.top_files(1)[0];
    assert_eq!(result.path(id).unwrap(), file);
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::process::Command;

    fn junction(link: &Path, target: &Path) {
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(status.status.success(), "mklink failed: {status:?}");
    }

    #[test]
    fn junctions_are_not_followed() {
        let dir = tree(&[("real/data.bin", 1000), ("other.bin", 10)]);
        junction(&dir.path().join("to-real"), &dir.path().join("real"));
        junction(&dir.path().join("real").join("to-ancestor"), dir.path());

        let result = run(dir.path());
        assert_eq!(result.state(), ScanState::Completed);
        assert_eq!(result.total_size(), 1010);
        assert_eq!(result.total_files(), 2);

        let link = result
            .node(child_named(&result, result.root(), "to-real"))
            .unwrap();
        assert_eq!(link.kind(), NodeKind::Folder);
        assert_eq!(link.size(), 0);
        assert_eq!(link.child_count(), 0);

        let real = child_named(&result, result.root(), "real");
        let ancestor = child_named(&result, real, "to-ancestor");
        assert_eq!(result.node(ancestor).unwrap().child_count(), 0);
    }

    /// Denies listing a folder to Everyone and restores it on drop.
    struct Denied(PathBuf);

    impl Denied {
        fn new(path: &Path) -> Self {
            let output = Command::new("icacls")
                .arg(path)
                .args(["/deny", "*S-1-1-0:(RD)"])
                .output()
                .unwrap();
            assert!(output.status.success(), "icacls failed: {output:?}");
            Self(path.to_path_buf())
        }
    }

    impl Drop for Denied {
        fn drop(&mut self) {
            let _ = Command::new("icacls")
                .arg(&self.0)
                .args(["/remove:d", "*S-1-1-0"])
                .output();
        }
    }

    #[test]
    fn unreadable_folders_are_recorded_and_the_rest_is_counted() {
        let dir = tree(&[("ok/a.bin", 100), ("locked/secret.bin", 999), ("b.bin", 5)]);
        let locked = dir.path().join("locked");
        let _denied = Denied::new(&locked);

        let result = run(dir.path());
        assert_eq!(result.state(), ScanState::Completed);
        assert_eq!(result.unreadable(), [locked]);
        assert_eq!(result.total_size(), 105);
        assert_eq!(result.total_files(), 2);
        assert_eq!(result.total_folders(), 2);
    }

    #[test]
    fn folders_queued_before_cancel_are_never_opened() {
        let dir = tree(&[
            ("a.bin", 10),
            ("open/x.bin", 1),
            ("zz-locked/secret.bin", 1),
        ]);
        let locked = dir.path().join("zz-locked");
        let _denied = Denied::new(&locked);

        // Cancelling on the first progress event (right after the Target is
        // listed) must not open any subfolder, so the locked one is never
        // tried and never reported as unreadable.
        let cancel = AtomicBool::new(false);
        let result = scan(
            dir.path(),
            &ScanOptions::default(),
            &|_| cancel.store(true, Ordering::Relaxed),
            &cancel,
        )
        .unwrap();

        assert_eq!(result.state(), ScanState::Cancelled);
        assert!(result.unreadable().is_empty(), "{:?}", result.unreadable());
    }

    #[test]
    fn unreadable_target_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        let _denied = Denied::new(&locked);

        let error = scan(
            &locked,
            &ScanOptions::default(),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(error, ScanError::TargetUnreadable { .. }));
    }
}
