//! Physical disks and their health, as Windows' storage management reports
//! it (`MSFT_PhysicalDisk`), and drives that are low on free space.

use serde::Deserialize;
use wmi::WMIConnection;

use crate::Unavailable;

/// Below this share of free space a drive is "low on space".
const LOW_SPACE_RATIO: f64 = 0.10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Ssd,
    Hdd,
    /// Storage-class memory.
    Scm,
    Unspecified,
}

/// The health Windows reports for a disk. `Warning` and `Unhealthy` mean the
/// disk may fail: the user should back up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskHealth {
    Healthy,
    Warning,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalDisk {
    pub id: String,
    pub model: String,
    pub media_type: MediaType,
    /// Connection, e.g. `NVMe`, `SATA`, `USB`.
    pub bus_type: String,
    pub size_bytes: u64,
    pub health: DiskHealth,
}

impl PhysicalDisk {
    pub fn has_problem(&self) -> bool {
        matches!(self.health, DiskHealth::Warning | DiskHealth::Unhealthy)
    }
}

#[derive(Deserialize)]
#[serde(rename = "MSFT_PhysicalDisk", rename_all = "PascalCase")]
struct MsftPhysicalDisk {
    device_id: Option<String>,
    friendly_name: Option<String>,
    model: Option<String>,
    media_type: Option<u16>,
    bus_type: Option<u16>,
    size: Option<u64>,
    health_status: Option<u16>,
}

pub fn list_physical_disks() -> Result<Vec<PhysicalDisk>, Unavailable> {
    crate::com::on_com_thread(|| {
        let connection = WMIConnection::with_namespace_path(r"ROOT\Microsoft\Windows\Storage")
            .map_err(unavailable)?;
        let disks: Vec<MsftPhysicalDisk> = connection.query().map_err(unavailable)?;
        let mut disks: Vec<PhysicalDisk> = disks.into_iter().map(PhysicalDisk::from).collect();
        disks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(disks)
    })
}

fn unavailable(error: wmi::WMIError) -> Unavailable {
    match error {
        wmi::WMIError::HResultError { hres } => Unavailable::from(
            windows::core::Error::from_hresult(windows::core::HRESULT(hres)),
        ),
        other => Unavailable::Failed(other.to_string()),
    }
}

impl From<MsftPhysicalDisk> for PhysicalDisk {
    fn from(disk: MsftPhysicalDisk) -> Self {
        Self {
            id: disk.device_id.unwrap_or_default(),
            model: disk
                .friendly_name
                .or(disk.model)
                .map(|m| m.trim().to_string())
                .unwrap_or_default(),
            media_type: match disk.media_type {
                Some(3) => MediaType::Hdd,
                Some(4) => MediaType::Ssd,
                Some(5) => MediaType::Scm,
                _ => MediaType::Unspecified,
            },
            bus_type: bus_type_name(disk.bus_type.unwrap_or(0)).to_string(),
            size_bytes: disk.size.unwrap_or(0),
            health: match disk.health_status {
                Some(0) => DiskHealth::Healthy,
                Some(1) => DiskHealth::Warning,
                Some(2) => DiskHealth::Unhealthy,
                _ => DiskHealth::Unknown,
            },
        }
    }
}

/// Names of `MSFT_PhysicalDisk.BusType` values.
fn bus_type_name(code: u16) -> &'static str {
    match code {
        1 => "SCSI",
        2 => "ATAPI",
        3 => "ATA",
        4 => "IEEE 1394",
        5 => "SSA",
        6 => "Fibre Channel",
        7 => "USB",
        8 => "RAID",
        9 => "iSCSI",
        10 => "SAS",
        11 => "SATA",
        12 => "SD",
        13 => "MMC",
        15 => "File Backed Virtual",
        16 => "Storage Spaces",
        17 => "NVMe",
        _ => "Unknown",
    }
}

/// A drive with its free space and whether it is low on space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub drive: scanner::Drive,
    pub low_space: bool,
}

/// `true` when less than 10% of the drive is free.
pub fn is_low_space(free_bytes: u64, total_bytes: u64) -> bool {
    total_bytes > 0 && (free_bytes as f64) < total_bytes as f64 * LOW_SPACE_RATIO
}

pub fn list_volumes() -> Vec<Volume> {
    scanner::list_drives()
        .into_iter()
        .map(|drive| Volume {
            low_space: is_low_space(drive.free_bytes, drive.total_bytes),
            drive,
        })
        .collect()
}
