//! Finding the program a command line runs, as Windows does for Run keys and
//! uninstall registrations: a quoted path, or the shortest unquoted prefix
//! that names an existing file.

use std::path::{Path, PathBuf};

use crate::wide::{from_wide, Wide};

/// A command line split into its program and the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    /// The program to run. It may not exist: callers check.
    pub program: PathBuf,
    pub arguments: String,
}

/// Splits `command` (after expanding environment variables) into its program
/// and arguments. Returns `None` for an empty command.
pub fn parse(command: &str) -> Option<CommandLine> {
    let expanded = crate::wide::expand_env(command);
    let command = expanded.trim();
    if command.is_empty() {
        return None;
    }

    if let Some(rest) = command.strip_prefix('"') {
        let (program, arguments) = match rest.find('"') {
            Some(end) => (&rest[..end], rest[end + 1..].trim()),
            None => (rest, ""),
        };
        return Some(CommandLine {
            program: resolve(program).unwrap_or_else(|| PathBuf::from(program)),
            arguments: arguments.to_string(),
        });
    }

    // Unquoted: `C:\Program Files\App\app.exe /x` is ambiguous, so try each
    // space-separated prefix, shortest first, like CreateProcess does.
    let spaces: Vec<usize> = command.match_indices(' ').map(|(i, _)| i).collect();
    for end in spaces.iter().copied().chain(Some(command.len())) {
        let candidate = &command[..end];
        if let Some(program) = resolve(candidate) {
            return Some(CommandLine {
                program,
                arguments: command[end..].trim().to_string(),
            });
        }
    }

    // Nothing exists: take the first word, so a missing program is reported.
    let end = spaces.first().copied().unwrap_or(command.len());
    Some(CommandLine {
        program: PathBuf::from(&command[..end]),
        arguments: command[end..].trim().to_string(),
    })
}

/// The existing file `candidate` refers to: as written, with `.exe` added, or
/// found on the search path when it is a bare name such as `rundll32.exe`.
fn resolve(candidate: &str) -> Option<PathBuf> {
    let path = Path::new(candidate);
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if path.extension().is_none() {
        let with_exe = PathBuf::from(format!("{candidate}.exe"));
        if with_exe.is_file() {
            return Some(with_exe);
        }
    }
    if !candidate.contains(['\\', '/', ':']) {
        return search_path(candidate);
    }
    None
}

fn search_path(name: &str) -> Option<PathBuf> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::SearchPathW;

    let file = Wide::new(name);
    let extension = Wide::new(".exe");
    let mut buffer = vec![0u16; 1024];
    // SAFETY: strings are null-terminated and the buffer size is passed.
    let len = unsafe {
        SearchPathW(
            PCWSTR::null(),
            file.as_pcwstr(),
            extension.as_pcwstr(),
            Some(&mut buffer),
            None,
        )
    };
    if len == 0 || len as usize >= buffer.len() {
        return None;
    }
    let path = PathBuf::from(from_wide(&buffer));
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn quoted_program_with_arguments() {
        let parsed = parse(r#""C:\No Such\app.exe" --minimized"#).unwrap();
        assert_eq!(parsed.program, PathBuf::from(r"C:\No Such\app.exe"));
        assert_eq!(parsed.arguments, "--minimized");
    }

    #[test]
    fn unquoted_path_with_spaces_finds_the_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("My App");
        std::fs::create_dir(&folder).unwrap();
        let exe = folder.join("run me.exe");
        File::create(&exe).unwrap();

        let command = format!("{} /background", exe.with_extension("").display());
        let parsed = parse(&command).unwrap();
        assert_eq!(parsed.program, exe);
        assert_eq!(parsed.arguments, "/background");
    }

    #[test]
    fn bare_name_is_found_on_the_search_path() {
        let parsed = parse("rundll32.exe shell32.dll,Control_RunDLL").unwrap();
        assert!(parsed.program.is_absolute(), "{:?}", parsed.program);
        assert!(parsed.program.is_file());
        assert_eq!(parsed.arguments, "shell32.dll,Control_RunDLL");
    }

    #[test]
    fn missing_program_keeps_the_first_word() {
        let parsed = parse(r"C:\nowhere\gone.exe -x").unwrap();
        assert_eq!(parsed.program, PathBuf::from(r"C:\nowhere\gone.exe"));
        assert_eq!(parsed.arguments, "-x");
    }

    #[test]
    fn environment_variables_are_expanded() {
        let parsed = parse(r"%SystemRoot%\System32\cmd.exe /c").unwrap();
        assert!(parsed.program.is_file(), "{:?}", parsed.program);
    }

    #[test]
    fn empty_command_has_no_program() {
        assert_eq!(parse("   "), None);
    }
}
