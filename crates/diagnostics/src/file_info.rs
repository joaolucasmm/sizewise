//! Strings from a program file's version resource (publisher, description),
//! the same ones Explorer shows in the file's Details tab.

use std::path::Path;

use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};

use crate::wide::{from_wide, Wide};

/// The version resource of a file, read once and queried for several fields.
pub struct VersionInfo {
    data: Vec<u8>,
    /// Language and code page of each string table, e.g. `040904b0`.
    tables: Vec<String>,
}

impl VersionInfo {
    pub fn read(path: &Path) -> Option<Self> {
        let file = Wide::new(path);
        // SAFETY: `file` is null-terminated; the handle out-param is optional.
        let size = unsafe { GetFileVersionInfoSizeW(file.as_pcwstr(), None) };
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        // SAFETY: `data` has the size Windows asked for.
        unsafe { GetFileVersionInfoW(file.as_pcwstr(), None, size, data.as_mut_ptr().cast()) }
            .ok()?;

        let mut tables = Vec::new();
        if let Some(raw) = query(&data, r"\VarFileInfo\Translation", false) {
            for [l0, l1, c0, c1] in raw.as_chunks::<4>().0 {
                let language = u16::from_le_bytes([*l0, *l1]);
                let code_page = u16::from_le_bytes([*c0, *c1]);
                tables.push(format!("{language:04x}{code_page:04x}"));
            }
        }
        // Common tables for files whose translation list is missing or wrong.
        for fallback in ["040904b0", "040904e4", "04090000"] {
            if !tables.iter().any(|t| t == fallback) {
                tables.push(fallback.to_string());
            }
        }
        Some(Self { data, tables })
    }

    /// A string field such as `CompanyName` or `FileDescription`.
    pub fn field(&self, name: &str) -> Option<String> {
        self.tables.iter().find_map(|table| {
            let raw = query(
                &self.data,
                &format!(r"\StringFileInfo\{table}\{name}"),
                true,
            )?;
            let units: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| u16::from_le_bytes(pair))
                .collect();
            let text = from_wide(&units).trim().to_string();
            (!text.is_empty()).then_some(text)
        })
    }
}

/// The bytes of one value in a version resource. String values report their
/// length in characters, the others in bytes.
fn query<'a>(data: &'a [u8], sub_block: &str, is_string: bool) -> Option<&'a [u8]> {
    let sub_block = Wide::new(sub_block);
    let mut pointer = std::ptr::null_mut();
    let mut len = 0u32;
    // SAFETY: `data` is a version resource read by GetFileVersionInfoW; the
    // returned pointer points inside it and lives as long as `data`.
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            sub_block.as_pcwstr(),
            &mut pointer,
            &mut len,
        )
    };
    if !found.as_bool() || pointer.is_null() || len == 0 {
        return None;
    }
    let start = pointer as usize - data.as_ptr() as usize;
    let bytes = if is_string {
        len as usize * 2
    } else {
        len as usize
    };
    data.get(start..(start + bytes).min(data.len()))
}

/// The publisher of a program file, from its version resource.
pub fn publisher(path: &Path) -> Option<String> {
    VersionInfo::read(path)?.field("CompanyName")
}
