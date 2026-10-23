//! Diagnoses of what is slowing the PC, for Sizewise.
//!
//! This crate has no Tauri dependency: it reads Startup items, samples running
//! processes, lists installed programs and physical disks, and reads Windows'
//! own startup measurements. Everything here is read-only except ending a
//! process and starting a program's own uninstaller, which only happen when the
//! user asks. Nothing is ever estimated: a value Windows does not provide is
//! reported as missing, with the reason.

#[cfg(not(windows))]
compile_error!("the diagnostics crate only supports Windows");

mod boot;
mod com;
mod command;
mod disks;
mod file_info;
mod overview;
mod processes;
mod programs;
mod startup;
mod tasks;
mod wide;

pub use boot::{startup_durations, StartupDuration};
pub use disks::{
    is_low_space, list_physical_disks, list_volumes, DiskHealth, MediaType, PhysicalDisk, Volume,
};
pub use overview::{overview, Overview};
pub use processes::{
    end_process, EndTaskError, Process, ProcessGroup, ProcessSampler, Protection, Snapshot,
    SystemTotals,
};
pub use programs::{
    list_installed_programs, measure_install_folder, start_uninstall, InstallDate,
    InstalledProgram, MeasureError, UninstallError, Uninstaller,
};
pub use startup::{list_startup_items, StartupItem, StartupSource, StartupState};

/// Whether a Diagnosis applies to the current user or to every user of the PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    CurrentUser,
    AllUsers,
}

/// Why a Diagnosis has no value: the app shows the reason instead of a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// Windows only gives this data to administrators.
    NeedsAdministrator,
    /// Windows returned an error; the message is for display.
    Failed(String),
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsAdministrator => f.write_str("needs administrator rights"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl From<windows::core::Error> for Unavailable {
    fn from(error: windows::core::Error) -> Self {
        use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, E_ACCESSDENIED};
        // WMI reports its own access-denied code (WBEM_E_ACCESS_DENIED).
        const WBEM_E_ACCESS_DENIED: i32 = 0x8004_1003_u32 as i32;
        let code = error.code();
        if code == E_ACCESSDENIED
            || code == ERROR_ACCESS_DENIED.to_hresult()
            || code.0 == WBEM_E_ACCESS_DENIED
        {
            Self::NeedsAdministrator
        } else {
            Self::Failed(error.message())
        }
    }
}
