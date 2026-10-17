use std::path::PathBuf;

/// A local drive that can be chosen as a Target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    /// Root path, e.g. `C:\`.
    pub root: PathBuf,
    /// Drive letter, e.g. `C`.
    pub letter: char,
    /// Volume label; empty when the volume has none.
    pub label: String,
    /// File system name, e.g. `NTFS`.
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

/// Lists the local fixed and removable drives that are ready to be read.
#[cfg(windows)]
pub fn list_drives() -> Vec<Drive> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;

    // SAFETY: no arguments; returns a bitmask of available drive letters.
    let mask = unsafe { GetLogicalDrives() };
    let mut drives = Vec::new();
    for (bit, letter) in ('A'..='Z').enumerate() {
        if mask & (1 << bit) == 0 {
            continue;
        }
        let root = format!("{letter}:\\");
        let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();

        // SAFETY: `wide` is a valid null-terminated UTF-16 string.
        let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let mut label = [0u16; 261];
        let mut file_system = [0u16; 261];
        // SAFETY: buffers are valid for the sizes passed; unused out-params are null.
        let has_volume = unsafe {
            GetVolumeInformationW(
                wide.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                file_system.as_mut_ptr(),
                file_system.len() as u32,
            )
        };
        if has_volume == 0 {
            // No media inserted (e.g. an empty card reader).
            continue;
        }

        let (mut total_bytes, mut free_bytes) = (0u64, 0u64);
        // SAFETY: out-params point to valid u64s; the unused one is null.
        let has_space = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                &mut total_bytes,
                &mut free_bytes,
            )
        };
        if has_space == 0 {
            continue;
        }

        drives.push(Drive {
            root: PathBuf::from(root),
            letter,
            label: from_wide(&label),
            file_system: from_wide(&file_system),
            total_bytes,
            free_bytes,
        });
    }
    drives
}

#[cfg(windows)]
fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

/// Sizewise targets Windows; other platforms have no drive list.
#[cfg(not(windows))]
pub fn list_drives() -> Vec<Drive> {
    Vec::new()
}
