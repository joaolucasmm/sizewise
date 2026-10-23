//! Installed (Win32) programs, from the uninstall registrations Windows'
//! "Installed apps" settings page also reads, and starting a program's own
//! uninstaller.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
};
use winreg::{RegKey, HKEY};

use crate::wide::Wide;
use crate::Scope;

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Registrations of these kinds are updates, not programs.
const UPDATE_RELEASE_TYPES: &[&str] = &[
    "Update",
    "Hotfix",
    "Security Update",
    "Update Rollup",
    "Service Pack",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledProgram {
    /// Stable id of the registration, e.g. `HKLM64\{GUID}`.
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_date: Option<InstallDate>,
    /// Size the program reported when it was installed; `None` when it
    /// reported none.
    pub size_bytes: Option<u64>,
    pub install_location: Option<PathBuf>,
    pub uninstall_command: String,
    pub scope: Scope,
}

/// Programs installed for all users (64- and 32-bit) and for the current
/// user. System components and updates are left out, and so are entries
/// with no uninstaller, since the user couldn't remove them anyway.
pub fn list_installed_programs() -> Vec<InstalledProgram> {
    let views: [(HKEY, u32, &str, Scope); 3] = [
        (
            HKEY_LOCAL_MACHINE,
            KEY_WOW64_64KEY,
            "HKLM64",
            Scope::AllUsers,
        ),
        (
            HKEY_LOCAL_MACHINE,
            KEY_WOW64_32KEY,
            "HKLM32",
            Scope::AllUsers,
        ),
        (
            HKEY_CURRENT_USER,
            KEY_WOW64_64KEY,
            "HKCU",
            Scope::CurrentUser,
        ),
    ];
    let mut seen = HashSet::new();
    let mut programs = Vec::new();
    for (root, view, prefix, scope) in views {
        let Ok(uninstall) = RegKey::predef(root).open_subkey_with_flags(UNINSTALL, KEY_READ | view)
        else {
            continue;
        };
        for key_name in uninstall.enum_keys().filter_map(Result::ok) {
            let Ok(key) = uninstall.open_subkey_with_flags(&key_name, KEY_READ | view) else {
                continue;
            };
            let Some(program) = read_program(&key, format!(r"{prefix}\{key_name}"), scope) else {
                continue;
            };
            // Some installers register the same program in several places.
            let identity = (
                program.name.to_lowercase(),
                program.version.clone(),
                program.publisher.clone(),
            );
            if seen.insert(identity) {
                programs.push(program);
            }
        }
    }
    programs
}

fn read_program(key: &RegKey, id: String, scope: Scope) -> Option<InstalledProgram> {
    let text = |name: &str| {
        key.get_value::<String, _>(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let number = |name: &str| key.get_value::<u32, _>(name).ok();

    if number("SystemComponent") == Some(1) || text("ParentKeyName").is_some() {
        return None;
    }
    if text("ReleaseType").is_some_and(|t| {
        UPDATE_RELEASE_TYPES
            .iter()
            .any(|u| u.eq_ignore_ascii_case(&t))
    }) {
        return None;
    }
    let name = text("DisplayName")?;
    let uninstall_command = text("UninstallString")?;

    Some(InstalledProgram {
        id,
        name,
        publisher: text("Publisher"),
        version: text("DisplayVersion"),
        install_date: text("InstallDate").as_deref().and_then(parse_install_date),
        // Reported in kilobytes.
        size_bytes: number("EstimatedSize")
            .filter(|&kb| kb > 0)
            .map(|kb| u64::from(kb) * 1024),
        install_location: text("InstallLocation")
            .map(|l| PathBuf::from(crate::wide::expand_env(l.trim_matches('"'))))
            .filter(|p| p.is_dir()),
        uninstall_command,
        scope,
    })
}

/// `20260115` → 2026-01-15.
fn parse_install_date(text: &str) -> Option<InstallDate> {
    if text.len() != 8 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let date = InstallDate {
        year: text[0..4].parse().ok()?,
        month: text[4..6].parse().ok()?,
        day: text[6..8].parse().ok()?,
    };
    ((1..=12).contains(&date.month) && (1..=31).contains(&date.day) && date.year >= 1980)
        .then_some(date)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureError {
    /// The program doesn't say where it is installed.
    NoInstallLocation,
    Failed(String),
}

impl std::fmt::Display for MeasureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInstallLocation => f.write_str("the program has no install folder"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for MeasureError {}

/// Size of the program's install folder, measured with a Standard scan.
pub fn measure_install_folder(program: &InstalledProgram) -> Result<u64, MeasureError> {
    let location = program
        .install_location
        .as_ref()
        .ok_or(MeasureError::NoInstallLocation)?;
    let options = scanner::ScanOptions {
        top_n: 0,
        ..Default::default()
    };
    scanner::scan(location, &options, &|_| {}, &AtomicBool::new(false))
        .map(|result| result.total_size())
        .map_err(|e| MeasureError::Failed(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UninstallError {
    /// The user said no to the administrator prompt.
    Cancelled,
    /// The registered uninstall command names a program that doesn't exist.
    UninstallerMissing,
    Failed(String),
}

impl std::fmt::Display for UninstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("cancelled"),
            Self::UninstallerMissing => f.write_str("the uninstaller no longer exists"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for UninstallError {}

/// A running uninstaller.
pub struct Uninstaller(Option<HANDLE>);

// SAFETY: a process handle can be waited on and closed from any thread.
unsafe impl Send for Uninstaller {}

impl Uninstaller {
    /// Blocks until the uninstaller exits; returns its exit code when known.
    pub fn wait(self) -> Option<u32> {
        let handle = self.0?;
        // SAFETY: the handle came from ShellExecuteExW and is still open.
        unsafe { WaitForSingleObject(handle, INFINITE) };
        let mut code = 0u32;
        // SAFETY: `code` is a valid out-param.
        unsafe { GetExitCodeProcess(handle, &mut code) }
            .ok()
            .map(|()| code)
    }
}

impl Drop for Uninstaller {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            // SAFETY: closed once.
            let _ = unsafe { CloseHandle(handle) };
        }
    }
}

/// Starts the program's own registered uninstaller. Windows shows its
/// administrator prompt when the uninstaller needs it. What the uninstaller
/// removes is up to the program; Sizewise can't undo it.
pub fn start_uninstall(program: &InstalledProgram) -> Result<Uninstaller, UninstallError> {
    let command = crate::command::parse(&program.uninstall_command)
        .ok_or(UninstallError::UninstallerMissing)?;
    if !command.program.is_file() {
        return Err(UninstallError::UninstallerMissing);
    }
    let verb = Wide::new("open");
    let file = Wide::new(&command.program);
    let parameters = Wide::new(&command.arguments);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: verb.as_pcwstr(),
        lpFile: file.as_pcwstr(),
        lpParameters: parameters.as_pcwstr(),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: every string outlives the call; `cbSize` is set.
    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
        if e.code() == ERROR_CANCELLED.to_hresult() {
            UninstallError::Cancelled
        } else {
            UninstallError::Failed(e.message())
        }
    })?;
    Ok(Uninstaller(
        (!info.hProcess.is_invalid()).then_some(info.hProcess),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_dates() {
        assert_eq!(
            parse_install_date("20260115"),
            Some(InstallDate {
                year: 2026,
                month: 1,
                day: 15
            })
        );
        assert_eq!(parse_install_date("2026-01-15"), None);
        assert_eq!(parse_install_date("20261315"), None);
        assert_eq!(parse_install_date(""), None);
    }
}
