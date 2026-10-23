//! Conversions between Rust strings and the UTF-16 strings of the Win32 API.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;

/// A null-terminated UTF-16 copy of a string, kept alive while its pointer is used.
pub struct Wide(Vec<u16>);

impl Wide {
    pub fn new(text: impl AsRef<OsStr>) -> Self {
        Self(text.as_ref().encode_wide().chain(Some(0)).collect())
    }

    pub fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR(self.0.as_ptr())
    }
}

/// Reads a UTF-16 buffer up to its first null.
pub fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

/// Expands `%VARIABLE%` references the way Windows does for registry commands.
pub fn expand_env(text: &str) -> String {
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

    if !text.contains('%') {
        return text.to_string();
    }
    let source = Wide::new(text);
    // SAFETY: `source` is null-terminated; a `None` buffer asks for the size.
    let needed = unsafe { ExpandEnvironmentStringsW(source.as_pcwstr(), None) };
    if needed == 0 {
        return text.to_string();
    }
    let mut buffer = vec![0u16; needed as usize];
    // SAFETY: `buffer` has the size Windows asked for.
    let written = unsafe { ExpandEnvironmentStringsW(source.as_pcwstr(), Some(&mut buffer)) };
    if written == 0 || written > needed {
        return text.to_string();
    }
    from_wide(&buffer)
}
