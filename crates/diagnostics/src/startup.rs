//! Startup items: everything Windows launches when the user signs in.
//!
//! Sources are the registry Run keys, the Startup folders, Store app startup
//! tasks and logon scheduled tasks. Enabled/disabled is read from the same
//! place Task Manager uses (`Explorer\StartupApproved`), so both agree.

use std::path::{Path, PathBuf};

use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
};
use winreg::types::FromRegValue;
use winreg::{RegKey, HKEY};

use crate::{command, file_info, Scope};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
const STORE_TASKS: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\SystemAppData";
const STORE_PACKAGES: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupSource {
    /// A value under a `Run` registry key. `wow64` is the 32-bit view of the
    /// machine-wide key (`WOW6432Node`).
    RegistryRun {
        scope: Scope,
        wow64: bool,
    },
    StartupFolder {
        scope: Scope,
    },
    StoreApp,
    /// A scheduled task with a logon trigger; `path` is its Task Scheduler path.
    ScheduledTask {
        path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupItem {
    pub name: String,
    pub publisher: Option<String>,
    /// The command or target as configured (Store apps: the app's task id).
    pub command: String,
    /// The program file that runs, when there is one.
    pub target: Option<PathBuf>,
    pub source: StartupSource,
    pub state: StartupState,
    /// Whether `target` exists; `None` when there is no file to check.
    pub target_exists: Option<bool>,
}

impl StartupItem {
    /// A leftover: its program file no longer exists.
    pub fn is_broken(&self) -> bool {
        self.target_exists == Some(false)
    }
}

/// Every Startup item of the current user and of the whole PC. A source that
/// can't be read is skipped; the others are still listed.
pub fn list_startup_items() -> Vec<StartupItem> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let mut items = Vec::new();
    items.extend(run_key(
        HKEY_CURRENT_USER,
        KEY_WOW64_64KEY,
        Scope::CurrentUser,
        false,
        "Run",
    ));
    items.extend(run_key(
        HKEY_LOCAL_MACHINE,
        KEY_WOW64_64KEY,
        Scope::AllUsers,
        false,
        "Run",
    ));
    items.extend(run_key(
        HKEY_LOCAL_MACHINE,
        KEY_WOW64_32KEY,
        Scope::AllUsers,
        true,
        "Run32",
    ));
    if let Some(folder) = known_folder("APPDATA") {
        items.extend(startup_folder(&folder, &hkcu, Scope::CurrentUser));
    }
    if let Some(folder) = known_folder("ProgramData") {
        items.extend(startup_folder(&folder, &hklm, Scope::AllUsers));
    }
    items.extend(store_apps(&hkcu));
    items.extend(crate::tasks::logon_tasks());

    for item in &mut items {
        if item.publisher.is_none() {
            item.publisher = item.target.as_deref().and_then(file_info::publisher);
        }
    }
    items
}

/// Values of one `Run` key, with their state from the matching
/// `StartupApproved` key (`Run` or `Run32`).
fn run_key(
    root: HKEY,
    view: u32,
    scope: Scope,
    wow64: bool,
    approved_name: &str,
) -> Vec<StartupItem> {
    let root_key = RegKey::predef(root);
    let Ok(key) = root_key.open_subkey_with_flags(RUN, KEY_READ | view) else {
        return Vec::new();
    };
    let approved = root_key
        .open_subkey_with_flags(
            format!(r"{APPROVED}\{approved_name}"),
            KEY_READ | KEY_WOW64_64KEY,
        )
        .ok();
    key.enum_values()
        .filter_map(Result::ok)
        .filter_map(|(name, value)| {
            let command = String::from_reg_value(&value).ok()?;
            let target = command::parse(&command).map(|c| c.program);
            Some(StartupItem {
                state: approved_state(approved.as_ref(), &name),
                target_exists: Some(target.as_deref().is_some_and(Path::is_file)),
                name,
                publisher: None,
                command,
                target,
                source: StartupSource::RegistryRun { scope, wow64 },
            })
        })
        .collect()
}

/// Task Manager writes a binary value whose first byte is even when the item
/// is enabled (`02`, `06`) and odd when disabled (`03`, `07`). No value means
/// the item was never toggled, so it is enabled.
fn approved_state(approved: Option<&RegKey>, name: &str) -> StartupState {
    let first_byte = approved
        .and_then(|key| key.get_raw_value(name).ok())
        .and_then(|value| value.bytes.first().copied());
    match first_byte {
        Some(byte) if byte & 1 == 1 => StartupState::Disabled,
        _ => StartupState::Enabled,
    }
}

fn known_folder(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable).map(PathBuf::from)
}

/// Files in a Startup folder. `base` is `%APPDATA%` or `%ProgramData%`.
fn startup_folder(base: &Path, approved_root: &RegKey, scope: Scope) -> Vec<StartupItem> {
    let folder = base.join(r"Microsoft\Windows\Start Menu\Programs\Startup");
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    let approved = approved_root
        .open_subkey_with_flags(
            format!(r"{APPROVED}\StartupFolder"),
            KEY_READ | KEY_WOW64_64KEY,
        )
        .ok();
    let files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("desktop.ini"))
        })
        .collect();
    let links = crate::com::on_com_thread(|| {
        files
            .iter()
            .map(|file| (file.clone(), shortcut_target(file)))
            .collect::<Vec<_>>()
    });
    links
        .into_iter()
        .map(|(file, link)| {
            let file_name = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let is_link = file
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
            let (command, target) = match (is_link, link) {
                (true, Some((target, arguments))) => {
                    let command = if arguments.is_empty() {
                        target.display().to_string()
                    } else {
                        format!("\"{}\" {arguments}", target.display())
                    };
                    (command, Some(target))
                }
                // A shortcut without a file target (e.g. to a URL or a folder).
                (true, None) => (file.display().to_string(), None),
                (false, _) => (file.display().to_string(), Some(file.clone())),
            };
            StartupItem {
                name: file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                publisher: None,
                state: approved_state(approved.as_ref(), &file_name),
                target_exists: target.as_deref().map(Path::is_file),
                command,
                target,
                source: StartupSource::StartupFolder { scope },
            }
        })
        .collect()
}

/// Target path and arguments of a `.lnk` shortcut. Must run on a COM thread.
fn shortcut_target(file: &Path) -> Option<(PathBuf, String)> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    if !file
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
    {
        return None;
    }
    let path = crate::wide::Wide::new(file);
    // SAFETY: standard COM calls on an initialized thread; buffers are sized.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>()
            .ok()?
            .Load(path.as_pcwstr(), STGM_READ)
            .ok()?;
        let mut target = vec![0u16; 32_768];
        link.GetPath(&mut target, std::ptr::null_mut(), 0).ok()?;
        let target = crate::wide::from_wide(&target);
        if target.is_empty() {
            return None;
        }
        let mut arguments = vec![0u16; 32_768];
        let arguments = link
            .GetArguments(&mut arguments)
            .map(|()| crate::wide::from_wide(&arguments))
            .unwrap_or_default();
        Some((PathBuf::from(target), arguments))
    }
}

/// Store (packaged) apps' startup tasks. Each task has a `State` value:
/// 2 and 4 are enabled, anything else (0, 1, 3) disabled.
fn store_apps(hkcu: &RegKey) -> Vec<StartupItem> {
    let Ok(families) = hkcu.open_subkey_with_flags(STORE_TASKS, KEY_READ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for family in families.enum_keys().filter_map(Result::ok) {
        let Ok(family_key) = families.open_subkey_with_flags(&family, KEY_READ) else {
            continue;
        };
        for task in family_key.enum_keys().filter_map(Result::ok) {
            let Ok(task_key) = family_key.open_subkey_with_flags(&task, KEY_READ) else {
                continue;
            };
            let Ok(state) = task_key.get_value::<u32, _>("State") else {
                continue;
            };
            items.push(StartupItem {
                name: store_app_name(hkcu, &family)
                    .unwrap_or_else(|| package_name(&family).to_string()),
                publisher: None,
                command: format!("{family}!{task}"),
                target: None,
                source: StartupSource::StoreApp,
                state: if state == 2 || state == 4 {
                    StartupState::Enabled
                } else {
                    StartupState::Disabled
                },
                target_exists: None,
            });
        }
    }
    items
}

/// `Microsoft.WindowsTerminal` from `Microsoft.WindowsTerminal_8wekyb3d8bbwe`.
fn package_name(family: &str) -> &str {
    family.split('_').next().unwrap_or(family)
}

/// The display name of a Store app, from its package registration.
fn store_app_name(hkcu: &RegKey, family: &str) -> Option<String> {
    let (name, publisher_id) = family.rsplit_once('_')?;
    let packages = hkcu.open_subkey_with_flags(STORE_PACKAGES, KEY_READ).ok()?;
    let full_name = packages.enum_keys().filter_map(Result::ok).find(|key| {
        key.starts_with(&format!("{name}_")) && key.ends_with(&format!("_{publisher_id}"))
    })?;
    let display: String = packages
        .open_subkey_with_flags(&full_name, KEY_READ)
        .ok()?
        .get_value("DisplayName")
        .ok()?;
    // Most packages give a resource reference instead of the name itself.
    let indirect = if display.starts_with('@') {
        display
    } else if let Some(resource) = display.strip_prefix("ms-resource:") {
        let resource = if resource.starts_with("//") {
            format!("ms-resource:{resource}")
        } else if resource.starts_with('/') {
            format!("ms-resource://{name}{resource}")
        } else {
            format!("ms-resource://{name}/Resources/{resource}")
        };
        format!("@{{{full_name}?{resource}}}")
    } else {
        return Some(display);
    };
    let source = crate::wide::Wide::new(&indirect);
    let mut buffer = vec![0u16; 1024];
    // SAFETY: `source` is null-terminated and the buffer size is passed.
    unsafe {
        windows::Win32::UI::Shell::SHLoadIndirectString(source.as_pcwstr(), &mut buffer, None)
    }
    .ok()?;
    let resolved = crate::wide::from_wide(&buffer);
    (!resolved.is_empty()).then_some(resolved)
}
