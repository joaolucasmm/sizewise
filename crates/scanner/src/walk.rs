use std::cmp::Reverse;
use std::collections::VecDeque;
use std::fs::{self, DirEntry, Metadata, ReadDir};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, UNIX_EPOCH};

use rayon::prelude::*;

use crate::result::{Node, NodeId, NodeKind, ScanResult, ScanState};

/// Minimum time between two progress events (about 10 per second).
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// How many Large files to keep in the result.
    pub top_n: usize,
    /// Number of worker threads; `None` uses one per logical CPU.
    pub threads: Option<usize>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            top_n: 100,
            threads: None,
        }
    }
}

/// A snapshot of a running Scan.
#[derive(Debug, Clone)]
pub struct Progress {
    pub current_folder: PathBuf,
    pub files: u64,
    pub bytes: u64,
    pub elapsed: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("target not found: {0}")]
    TargetNotFound(PathBuf),
    #[error("target is not a folder: {0}")]
    NotAFolder(PathBuf),
    #[error("target cannot be read: {path}: {source}")]
    TargetUnreadable { path: PathBuf, source: io::Error },
    #[error("could not start the scan threads: {0}")]
    Threads(String),
}

/// Runs a Standard scan of `target`.
///
/// `on_progress` is called at most about 10 times per second while the Scan
/// runs, plus once at the end with the final totals. Setting `cancel` stops
/// the Scan cooperatively; the partial result comes back in the
/// [`ScanState::Cancelled`] state.
///
/// Junctions, symbolic links and mount points are not followed: they appear
/// with their own size, without descending. Folders that cannot be read are
/// listed in [`ScanResult::unreadable`] and the Scan continues.
pub fn scan(
    target: &Path,
    options: &ScanOptions,
    on_progress: &(dyn Fn(&Progress) + Sync),
    cancel: &AtomicBool,
) -> Result<ScanResult, ScanError> {
    let start = Instant::now();
    let metadata = fs::metadata(target).map_err(|source| match source.kind() {
        io::ErrorKind::NotFound => ScanError::TargetNotFound(target.to_path_buf()),
        _ => ScanError::TargetUnreadable {
            path: target.to_path_buf(),
            source,
        },
    })?;
    if !metadata.is_dir() {
        return Err(ScanError::NotAFolder(target.to_path_buf()));
    }
    let entries = fs::read_dir(target).map_err(|source| ScanError::TargetUnreadable {
        path: target.to_path_buf(),
        source,
    })?;

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads.unwrap_or(0))
        .stack_size(8 * 1024 * 1024)
        .thread_name(|i| format!("sizewise-scan-{i}"))
        .build()
        .map_err(|e| ScanError::Threads(e.to_string()))?;

    let walker = Walker {
        cancel,
        on_progress,
        start,
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
        last_report_ms: AtomicU64::new(0),
        unreadable: Mutex::new(Vec::new()),
    };
    let name = target.to_string_lossy().into();
    let root = pool.install(|| walker.walk_dir(target, name, modified_ms(&metadata), entries));

    let state = if cancel.load(Ordering::Relaxed) {
        ScanState::Cancelled
    } else {
        ScanState::Completed
    };
    walker.report(target);

    let nodes = flatten(root);
    let top_files = top_files(&nodes, options.top_n);
    let mut unreadable = walker
        .unreadable
        .into_inner()
        .unwrap_or_else(|e| e.into_inner());
    unreadable.sort();

    Ok(ScanResult {
        target: target.to_path_buf(),
        nodes,
        top_files,
        unreadable,
        duration: start.elapsed(),
        state,
    })
}

/// A node while the tree is being built; flattened into [`Node`]s at the end.
struct TempNode {
    name: Box<str>,
    kind: NodeKind,
    size: u64,
    files: u64,
    folders: u64,
    modified: Option<i64>,
    children: Vec<TempNode>,
}

impl TempNode {
    fn leaf(name: Box<str>, kind: NodeKind, size: u64, modified: Option<i64>) -> Self {
        Self {
            name,
            kind,
            size,
            files: 0,
            folders: 0,
            modified,
            children: Vec::new(),
        }
    }
}

struct Walker<'a> {
    cancel: &'a AtomicBool,
    on_progress: &'a (dyn Fn(&Progress) + Sync),
    start: Instant,
    files: AtomicU64,
    bytes: AtomicU64,
    last_report_ms: AtomicU64,
    unreadable: Mutex<Vec<PathBuf>>,
}

enum EntryKind {
    Folder,
    File,
    /// A junction, symbolic link or mount point: shown, never followed.
    Link(NodeKind),
}

impl Walker<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn visit_dir(&self, path: &Path, name: Box<str>, modified: Option<i64>) -> TempNode {
        match fs::read_dir(path) {
            Ok(entries) => self.walk_dir(path, name, modified, entries),
            Err(_) => {
                self.mark_unreadable(path);
                TempNode::leaf(name, NodeKind::Folder, 0, modified)
            }
        }
    }

    fn walk_dir(
        &self,
        path: &Path,
        name: Box<str>,
        modified: Option<i64>,
        entries: ReadDir,
    ) -> TempNode {
        let mut children = Vec::new();
        let mut subdirs = Vec::new();
        let mut files = 0;
        let mut bytes = 0;
        let mut entry_error = false;

        for entry in entries {
            if self.cancelled() {
                break;
            }
            let Some((entry, metadata)) = entry.ok().and_then(with_metadata) else {
                entry_error = true;
                continue;
            };
            let name: Box<str> = entry.file_name().to_string_lossy().into();
            let modified = modified_ms(&metadata);
            match classify(&entry, &metadata) {
                EntryKind::Folder => subdirs.push((entry.path(), name, modified)),
                EntryKind::File => {
                    files += 1;
                    bytes += metadata.len();
                    children.push(TempNode::leaf(
                        name,
                        NodeKind::File,
                        metadata.len(),
                        modified,
                    ));
                }
                EntryKind::Link(kind) => {
                    let size = if kind == NodeKind::File {
                        metadata.len()
                    } else {
                        0
                    };
                    if kind == NodeKind::File {
                        files += 1;
                        bytes += size;
                    }
                    children.push(TempNode::leaf(name, kind, size, modified));
                }
            }
        }
        if entry_error {
            self.mark_unreadable(path);
        }

        self.files.fetch_add(files, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.maybe_report(path);

        let subfolders: Vec<TempNode> = subdirs
            .into_par_iter()
            .map(|(path, name, modified)| self.visit_dir(&path, name, modified))
            .collect();
        children.extend(subfolders);

        let mut node = TempNode::leaf(name, NodeKind::Folder, 0, modified);
        for child in &children {
            node.size += child.size;
            node.files += child.files + u64::from(child.kind == NodeKind::File);
            node.folders += child.folders + u64::from(child.kind == NodeKind::Folder);
        }
        children.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        node.children = children;
        node
    }

    fn mark_unreadable(&self, path: &Path) {
        let mut unreadable = self.unreadable.lock().unwrap_or_else(|e| e.into_inner());
        unreadable.push(path.to_path_buf());
    }

    fn maybe_report(&self, current_folder: &Path) {
        let now = self.start.elapsed().as_millis() as u64;
        let last = self.last_report_ms.load(Ordering::Relaxed);
        if now.saturating_sub(last) < PROGRESS_INTERVAL.as_millis() as u64 {
            return;
        }
        // Only the thread that wins the race reports, so events stay rate-limited.
        if self
            .last_report_ms
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.report(current_folder);
        }
    }

    fn report(&self, current_folder: &Path) {
        (self.on_progress)(&Progress {
            current_folder: current_folder.to_path_buf(),
            files: self.files.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            elapsed: self.start.elapsed(),
        });
    }
}

fn with_metadata(entry: DirEntry) -> Option<(DirEntry, Metadata)> {
    // On Windows this comes from the directory listing itself: no extra system call.
    let metadata = entry.metadata().ok()?;
    Some((entry, metadata))
}

fn classify(entry: &DirEntry, metadata: &Metadata) -> EntryKind {
    // `is_symlink` covers every name-surrogate reparse point on Windows
    // (symbolic links, junctions, mount points), but not cloud placeholders
    // such as OneDrive folders, which are walked normally.
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        EntryKind::Link(if is_dir_link(entry, metadata) {
            NodeKind::Folder
        } else {
            NodeKind::File
        })
    } else if file_type.is_dir() {
        EntryKind::Folder
    } else {
        EntryKind::File
    }
}

#[cfg(windows)]
fn is_dir_link(_entry: &DirEntry, metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
}

#[cfg(not(windows))]
fn is_dir_link(entry: &DirEntry, _metadata: &Metadata) -> bool {
    fs::metadata(entry.path()).is_ok_and(|m| m.is_dir())
}

fn modified_ms(metadata: &Metadata) -> Option<i64> {
    let modified = metadata.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).ok(),
        Err(before) => i64::try_from(before.duration().as_millis())
            .ok()
            .map(|ms| -ms),
    }
}

/// Lays the tree out breadth-first so the children of each node are
/// contiguous and already in display order.
fn flatten(root: TempNode) -> Vec<Node> {
    let total = 1 + root.files + root.folders;
    let mut nodes = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    let mut queue = VecDeque::new();
    let mut root = root;
    nodes.push(to_node(&mut root, None));
    queue.push_back((root, 0 as NodeId));

    while let Some((mut temp, id)) = queue.pop_front() {
        let first_child = nodes.len() as NodeId;
        nodes[id as usize].first_child = first_child;
        nodes[id as usize].child_count = temp.children.len() as u32;
        for (i, mut child) in temp.children.drain(..).enumerate() {
            nodes.push(to_node(&mut child, Some(id)));
            if !child.children.is_empty() {
                queue.push_back((child, first_child + i as NodeId));
            }
        }
    }
    nodes
}

fn to_node(temp: &mut TempNode, parent: Option<NodeId>) -> Node {
    Node {
        name: std::mem::take(&mut temp.name),
        kind: temp.kind,
        size: temp.size,
        files: temp.files,
        folders: temp.folders,
        modified: temp.modified,
        parent,
        first_child: 0,
        child_count: 0,
    }
}

fn top_files(nodes: &[Node], n: usize) -> Vec<NodeId> {
    let key = |id: &NodeId| (Reverse(nodes[*id as usize].size), *id);
    let mut files: Vec<NodeId> = (0..nodes.len() as NodeId)
        .filter(|&id| nodes[id as usize].kind == NodeKind::File)
        .collect();
    if n == 0 {
        return Vec::new();
    }
    if files.len() > n {
        files.select_nth_unstable_by_key(n - 1, key);
        files.truncate(n);
    }
    files.sort_unstable_by_key(key);
    files
}
