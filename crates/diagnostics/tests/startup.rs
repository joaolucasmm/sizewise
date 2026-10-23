//! Startup items read from the real registry. Each test adds its own value to
//! the current user's Run key and always removes it, even when it fails.

use std::path::PathBuf;

use diagnostics::{
    list_startup_items, startup_durations, Scope, StartupItem, StartupSource, StartupState,
    Unavailable,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// A value in HKCU\...\Run that is deleted on drop.
struct RunValue {
    name: String,
}

impl RunValue {
    fn add(test: &str, command: &str) -> Self {
        let name = format!("Sizewise test {test} {}", std::process::id());
        // A fresh account (e.g. on a CI runner) may have no Run key yet.
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey_with_flags(RUN, KEY_SET_VALUE)
            .unwrap();
        key.set_value(&name, &command.to_string()).unwrap();
        Self { name }
    }

    fn remove(&self) {
        let _ = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN, KEY_SET_VALUE)
            .and_then(|key| key.delete_value(&self.name));
    }
}

impl Drop for RunValue {
    fn drop(&mut self) {
        self.remove();
    }
}

fn find(name: &str) -> Option<StartupItem> {
    list_startup_items()
        .into_iter()
        .find(|item| item.name == name)
}

fn system_program(file: &str) -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap())
        .join("System32")
        .join(file)
}

#[test]
fn a_run_key_item_is_listed_and_gone_after_removal() {
    let program = system_program("notepad.exe");
    let value = RunValue::add("listed", &format!("\"{}\" /x", program.display()));

    let item = find(&value.name).expect("item listed");
    assert_eq!(
        item.source,
        StartupSource::RegistryRun {
            scope: Scope::CurrentUser,
            wow64: false
        }
    );
    assert_eq!(item.state, StartupState::Enabled);
    assert_eq!(item.target.as_deref(), Some(program.as_path()));
    assert_eq!(item.target_exists, Some(true));
    assert!(!item.is_broken());
    assert!(item.publisher.is_some(), "notepad has a publisher");

    value.remove();
    assert!(find(&value.name).is_none());
}

#[test]
fn an_item_whose_program_is_missing_is_broken() {
    let value = RunValue::add("broken", r#""C:\Sizewise\does not exist\gone.exe" --start"#);

    let item = find(&value.name).expect("item listed");
    assert_eq!(item.target_exists, Some(false));
    assert!(item.is_broken());
}

#[test]
fn every_item_is_well_formed() {
    for item in list_startup_items() {
        assert!(!item.name.is_empty(), "{item:?}");
        assert!(!item.command.is_empty(), "{item:?}");
        match item.source {
            StartupSource::StoreApp => assert_eq!(item.target_exists, None),
            StartupSource::RegistryRun { .. } | StartupSource::ScheduledTask { .. } => {
                assert!(item.target_exists.is_some(), "{item:?}")
            }
            StartupSource::StartupFolder { .. } => {}
        }
    }
}

#[test]
fn startup_durations_are_measured_or_say_why_not() {
    match startup_durations(5) {
        Ok(durations) => {
            assert!(durations.len() <= 5);
            for duration in &durations {
                assert!(duration.boot_ms > 0);
                assert!(duration.recorded_at_ms > 0);
            }
            assert!(durations
                .windows(2)
                .all(|w| w[0].recorded_at_ms >= w[1].recorded_at_ms));
        }
        Err(Unavailable::NeedsAdministrator) => {}
        Err(other) => panic!("unexpected: {other}"),
    }
}
