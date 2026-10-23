//! Shapes of Diagnoses sent to the UI. Field names are camelCase on the
//! TypeScript side.

use diagnostics::{
    DiskHealth, EndTaskError, InstalledProgram, MediaType, Overview, PhysicalDisk, Process,
    ProcessGroup, Protection, Scope, Snapshot, StartupDuration, StartupItem, StartupSource,
    StartupState, SystemTotals, Unavailable, UninstallError, Volume,
};
use serde::Serialize;

use crate::dto::DriveDto;

fn path_string(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum UnavailableReason {
    NeedsAdministrator,
    Failed,
}

/// Why a Diagnosis has no value.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UnavailableDto {
    pub reason: UnavailableReason,
    pub message: String,
}

impl From<&Unavailable> for UnavailableDto {
    fn from(unavailable: &Unavailable) -> Self {
        Self {
            reason: match unavailable {
                Unavailable::NeedsAdministrator => UnavailableReason::NeedsAdministrator,
                Unavailable::Failed(_) => UnavailableReason::Failed,
            },
            message: unavailable.to_string(),
        }
    }
}

/// A value, or the reason it is missing; exactly one of the two is set.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability<T> {
    pub value: Option<T>,
    pub unavailable: Option<UnavailableDto>,
}

impl<T> Availability<T> {
    pub fn new<U>(result: &Result<U, Unavailable>, map: impl FnOnce(&U) -> T) -> Self {
        match result {
            Ok(value) => Self {
                value: Some(map(value)),
                unavailable: None,
            },
            Err(unavailable) => Self {
                value: None,
                unavailable: Some(unavailable.into()),
            },
        }
    }
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ScopeDto {
    CurrentUser,
    AllUsers,
}

impl From<Scope> for ScopeDto {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::CurrentUser => Self::CurrentUser,
            Scope::AllUsers => Self::AllUsers,
        }
    }
}

// Startup items

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StartupSourceDto {
    RegistryRun { scope: ScopeDto, wow64: bool },
    StartupFolder { scope: ScopeDto },
    StoreApp,
    ScheduledTask { path: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItemDto {
    pub name: String,
    pub publisher: Option<String>,
    pub command: String,
    pub target: Option<String>,
    pub source: StartupSourceDto,
    pub enabled: bool,
    pub broken: bool,
}

impl From<StartupItem> for StartupItemDto {
    fn from(item: StartupItem) -> Self {
        Self {
            broken: item.is_broken(),
            enabled: item.state == StartupState::Enabled,
            source: match item.source {
                StartupSource::RegistryRun { scope, wow64 } => StartupSourceDto::RegistryRun {
                    scope: scope.into(),
                    wow64,
                },
                StartupSource::StartupFolder { scope } => StartupSourceDto::StartupFolder {
                    scope: scope.into(),
                },
                StartupSource::StoreApp => StartupSourceDto::StoreApp,
                StartupSource::ScheduledTask { path } => StartupSourceDto::ScheduledTask { path },
            },
            target: item.target.as_deref().map(path_string),
            name: item.name,
            publisher: item.publisher,
            command: item.command,
        }
    }
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct StartupDurationDto {
    pub recorded_at: i64,
    pub boot_ms: u64,
    pub main_path_ms: u64,
    pub post_boot_ms: u64,
}

impl From<&StartupDuration> for StartupDurationDto {
    fn from(d: &StartupDuration) -> Self {
        Self {
            recorded_at: d.recorded_at_ms,
            boot_ms: d.boot_ms,
            main_path_ms: d.main_path_ms,
            post_boot_ms: d.post_boot_ms,
        }
    }
}

// Processes

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ProtectionDto {
    Critical,
    OwnProcess,
}

impl From<Protection> for ProtectionDto {
    fn from(protection: Protection) -> Self {
        match protection {
            Protection::Critical => Self::Critical,
            Protection::OwnProcess => Self::OwnProcess,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDto {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub cpu_percent: Option<f64>,
    pub ram_bytes: Option<u64>,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
    pub protection: Option<ProtectionDto>,
}

impl From<&Process> for ProcessDto {
    fn from(p: &Process) -> Self {
        Self {
            pid: p.pid,
            name: p.name.clone(),
            path: p.path.as_deref().map(path_string),
            cpu_percent: p.cpu_percent,
            ram_bytes: p.ram_bytes,
            read_bytes_per_sec: p.read_bytes_per_sec,
            write_bytes_per_sec: p.write_bytes_per_sec,
            protection: p.protection.map(Into::into),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessGroupDto {
    pub key: String,
    pub name: String,
    pub path: Option<String>,
    pub processes: Vec<ProcessDto>,
    pub cpu_percent: Option<f64>,
    pub ram_bytes: Option<u64>,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
}

impl From<&ProcessGroup> for ProcessGroupDto {
    fn from(g: &ProcessGroup) -> Self {
        Self {
            key: g.key.clone(),
            name: g.name.clone(),
            path: g.path.as_deref().map(path_string),
            processes: g.processes.iter().map(Into::into).collect(),
            cpu_percent: g.cpu_percent,
            ram_bytes: g.ram_bytes,
            read_bytes_per_sec: g.read_bytes_per_sec,
            write_bytes_per_sec: g.write_bytes_per_sec,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TotalsDto {
    pub cpu_percent: Option<f64>,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
}

impl From<&SystemTotals> for TotalsDto {
    fn from(t: &SystemTotals) -> Self {
        Self {
            cpu_percent: t.cpu_percent,
            ram_used_bytes: t.ram_used_bytes,
            ram_total_bytes: t.ram_total_bytes,
            read_bytes_per_sec: t.read_bytes_per_sec,
            write_bytes_per_sec: t.write_bytes_per_sec,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    pub groups: Vec<ProcessGroupDto>,
    pub totals: TotalsDto,
}

impl From<&Snapshot> for SnapshotDto {
    fn from(s: &Snapshot) -> Self {
        Self {
            groups: s.groups.iter().map(Into::into).collect(),
            totals: (&s.totals).into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndTaskReason {
    Critical,
    OwnProcess,
    AccessDenied,
    AlreadyExited,
    Failed,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndTaskFailureDto {
    pub reason: EndTaskReason,
    pub message: String,
}

impl From<EndTaskError> for EndTaskFailureDto {
    fn from(error: EndTaskError) -> Self {
        Self {
            reason: match &error {
                EndTaskError::Refused(Protection::Critical) => EndTaskReason::Critical,
                EndTaskError::Refused(Protection::OwnProcess) => EndTaskReason::OwnProcess,
                EndTaskError::AccessDenied => EndTaskReason::AccessDenied,
                EndTaskError::AlreadyExited => EndTaskReason::AlreadyExited,
                EndTaskError::Failed(_) => EndTaskReason::Failed,
            },
            message: error.to_string(),
        }
    }
}

// Installed programs

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramDto {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    /// `YYYY-MM-DD`.
    pub install_date: Option<String>,
    pub size_bytes: Option<u64>,
    pub install_location: Option<String>,
    pub scope: ScopeDto,
}

impl From<InstalledProgram> for ProgramDto {
    fn from(p: InstalledProgram) -> Self {
        Self {
            install_date: p
                .install_date
                .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)),
            install_location: p.install_location.as_deref().map(path_string),
            scope: p.scope.into(),
            id: p.id,
            name: p.name,
            publisher: p.publisher,
            version: p.version,
            size_bytes: p.size_bytes,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UninstallReason {
    Cancelled,
    UninstallerMissing,
    NotFound,
    Failed,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallFailureDto {
    pub reason: UninstallReason,
    pub message: String,
}

impl UninstallFailureDto {
    pub fn not_found() -> Self {
        Self {
            reason: UninstallReason::NotFound,
            message: "the program is no longer installed".to_string(),
        }
    }
}

impl From<UninstallError> for UninstallFailureDto {
    fn from(error: UninstallError) -> Self {
        Self {
            reason: match &error {
                UninstallError::Cancelled => UninstallReason::Cancelled,
                UninstallError::UninstallerMissing => UninstallReason::UninstallerMissing,
                UninstallError::Failed(_) => UninstallReason::Failed,
            },
            message: error.to_string(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UninstallFinishedDto {
    pub id: String,
    pub exit_code: Option<u32>,
}

// Disks

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalDiskDto {
    pub id: String,
    pub model: String,
    pub media_type: &'static str,
    pub bus_type: String,
    pub size_bytes: u64,
    pub health: &'static str,
    pub has_problem: bool,
}

impl From<&PhysicalDisk> for PhysicalDiskDto {
    fn from(d: &PhysicalDisk) -> Self {
        Self {
            id: d.id.clone(),
            model: d.model.clone(),
            media_type: match d.media_type {
                MediaType::Ssd => "ssd",
                MediaType::Hdd => "hdd",
                MediaType::Scm => "scm",
                MediaType::Unspecified => "unspecified",
            },
            bus_type: d.bus_type.clone(),
            size_bytes: d.size_bytes,
            health: match d.health {
                DiskHealth::Healthy => "healthy",
                DiskHealth::Warning => "warning",
                DiskHealth::Unhealthy => "unhealthy",
                DiskHealth::Unknown => "unknown",
            },
            has_problem: d.has_problem(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeDto {
    #[serde(flatten)]
    pub drive: DriveDto,
    pub low_space: bool,
}

impl From<Volume> for VolumeDto {
    fn from(v: Volume) -> Self {
        Self {
            drive: v.drive.into(),
            low_space: v.low_space,
        }
    }
}

pub fn disks_dto(
    disks: &Result<Vec<PhysicalDisk>, Unavailable>,
) -> Availability<Vec<PhysicalDiskDto>> {
    Availability::new(disks, |disks| disks.iter().map(Into::into).collect())
}

// Overview

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewDto {
    pub volumes: Vec<VolumeDto>,
    pub enabled_startup_items: usize,
    pub last_startup: Availability<Option<StartupDurationDto>>,
    pub disks: Availability<Vec<PhysicalDiskDto>>,
}

impl From<Overview> for OverviewDto {
    fn from(o: Overview) -> Self {
        Self {
            last_startup: Availability::new(&o.last_startup, |d| d.as_ref().map(Into::into)),
            disks: disks_dto(&o.disks),
            volumes: o.volumes.into_iter().map(Into::into).collect(),
            enabled_startup_items: o.enabled_startup_items,
        }
    }
}
