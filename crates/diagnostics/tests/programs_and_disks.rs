//! Installed programs and disks, read from the real Windows.

use diagnostics::{
    is_low_space, list_installed_programs, list_physical_disks, list_volumes, Unavailable,
};

#[test]
fn installed_programs_are_well_formed() {
    let programs = list_installed_programs();
    for program in &programs {
        assert!(!program.name.trim().is_empty(), "{program:?}");
        assert!(!program.uninstall_command.is_empty(), "{program:?}");
        assert!(!program.id.is_empty());
        // Updates are left out.
        assert!(
            !program.name.starts_with("Security Update for"),
            "{program:?}"
        );
        if let Some(date) = program.install_date {
            assert!((1..=12).contains(&date.month) && (1..=31).contains(&date.day));
        }
        if let Some(location) = &program.install_location {
            assert!(location.is_dir());
        }
    }
    let mut ids: Vec<&str> = programs.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), programs.len(), "ids are unique");
}

#[test]
fn physical_disks_are_listed_or_say_why_not() {
    match list_physical_disks() {
        Ok(disks) => {
            assert!(!disks.is_empty());
            for disk in &disks {
                assert!(disk.size_bytes > 0, "{disk:?}");
            }
        }
        Err(Unavailable::NeedsAdministrator) => {}
        Err(Unavailable::Failed(message)) => assert!(!message.is_empty()),
    }
}

#[test]
fn system_drive_is_listed_with_consistent_space() {
    let system_root = std::env::var("SystemDrive").unwrap(); // e.g. "C:"
    let letter = system_root.chars().next().unwrap().to_ascii_uppercase();
    let volume = list_volumes()
        .into_iter()
        .find(|v| v.drive.letter == letter)
        .expect("system drive listed");
    assert!(volume.drive.total_bytes > 0);
    assert!(volume.drive.free_bytes <= volume.drive.total_bytes);
    assert_eq!(
        volume.low_space,
        is_low_space(volume.drive.free_bytes, volume.drive.total_bytes)
    );
}

#[test]
fn low_space_means_under_ten_percent_free() {
    assert!(is_low_space(99, 1000));
    assert!(!is_low_space(100, 1000));
    assert!(!is_low_space(500, 1000));
    assert!(is_low_space(0, 1000));
    // An empty or unreadable drive is not reported as low.
    assert!(!is_low_space(0, 0));
}
