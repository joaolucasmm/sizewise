//! Resource monitor: snapshots of running processes and ending one on request.
//!
//! CPU and I/O rates come from the difference between two samples, so the
//! first snapshot of a sampler has no rates. RAM is the private working set,
//! the value Task Manager shows in its Memory column. I/O is what Windows
//! counts per process: file and disk reads and writes, plus other device I/O.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, FILETIME, HANDLE, STILL_ACTIVE,
    WAIT_OBJECT_0,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessIoCounters, GetProcessTimes, GetSystemTimes, IsProcessCritical,
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, IO_COUNTERS,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

use crate::file_info::VersionInfo;
use crate::wide::from_wide;

/// Processes that keep Windows running; ending any of them can crash the PC
/// or sign the user out. Compared case-insensitively with the process name.
const CRITICAL_NAMES: &[&str] = &[
    "system idle process",
    "system",
    "secure system",
    "registry",
    "memory compression",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "lsaiso.exe",
    "svchost.exe",
];

/// Why "End task" is not offered for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// A process Windows needs to keep running.
    Critical,
    /// Sizewise itself.
    OwnProcess,
}

/// One running process. Values Windows doesn't give for a process (usually
/// because it belongs to another user or to the system) are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub parent_pid: u32,
    /// Executable file name, e.g. `chrome.exe`.
    pub name: String,
    pub path: Option<PathBuf>,
    /// Share of the whole CPU (all cores) since the previous sample, 0–100.
    pub cpu_percent: Option<f64>,
    /// Private working set, in bytes.
    pub ram_bytes: Option<u64>,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
    /// Processes running the same executable share this key.
    pub group_key: String,
    pub protection: Option<Protection>,
}

/// The processes of one program (same executable), with their combined usage.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessGroup {
    pub key: String,
    /// The program's description (e.g. "Google Chrome"), or its file name.
    pub name: String,
    pub path: Option<PathBuf>,
    pub processes: Vec<Process>,
    pub cpu_percent: Option<f64>,
    /// `None` when no process of the group could be read.
    pub ram_bytes: Option<u64>,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
}

/// Usage of the whole PC.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemTotals {
    /// Share of the whole CPU in use since the previous sample, 0–100.
    pub cpu_percent: Option<f64>,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    /// Sum of every process's I/O rate.
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
    pub logical_cpus: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Groups in no particular order; the UI sorts them.
    pub groups: Vec<ProcessGroup>,
    pub totals: SystemTotals,
}

impl Snapshot {
    pub fn processes(&self) -> impl Iterator<Item = &Process> {
        self.groups.iter().flat_map(|g| g.processes.iter())
    }
}

/// Raw counters of one process at one moment.
#[derive(Clone, Copy)]
struct Counters {
    cpu_time: u64,
    read_bytes: u64,
    write_bytes: u64,
}

#[derive(Clone, Copy)]
struct SystemTimes {
    idle: u64,
    busy_and_idle: u64,
}

struct Previous {
    at: Instant,
    system: Option<SystemTimes>,
    /// Keyed by pid and creation time, so a reused pid is not mistaken for
    /// the process that had it before.
    processes: HashMap<(u32, u64), Counters>,
}

/// Produces Snapshots; keeps the previous sample to compute rates.
#[derive(Default)]
pub struct ProcessSampler {
    previous: Option<Previous>,
    descriptions: HashMap<PathBuf, Option<String>>,
}

impl ProcessSampler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let logical_cpus = logical_cpus();
        let elapsed = self
            .previous
            .as_ref()
            .map(|p| now.duration_since(p.at).as_secs_f64())
            .filter(|&s| s > 0.0);

        let system = system_times();
        let system_cpu = match (&self.previous, system) {
            (
                Some(Previous {
                    system: Some(before),
                    ..
                }),
                Some(after),
            ) => {
                let total = after.busy_and_idle.saturating_sub(before.busy_and_idle);
                let idle = after.idle.saturating_sub(before.idle);
                (total > 0).then(|| total.saturating_sub(idle) as f64 / total as f64 * 100.0)
            }
            _ => None,
        };

        let own_pid = std::process::id();
        let mut counters = HashMap::new();
        let mut processes = Vec::new();
        for entry in process_entries() {
            let details = read_details(entry.pid);
            let previous = details.as_ref().and_then(|d| {
                let key = (entry.pid, d.creation_time);
                counters.insert(key, d.counters);
                self.previous.as_ref()?.processes.get(&key).copied()
            });
            let rates = match (&details, previous, elapsed) {
                (Some(d), Some(before), Some(seconds)) => {
                    Some(rates(&before, &d.counters, seconds, logical_cpus))
                }
                _ => None,
            };
            let path = details.as_ref().and_then(|d| d.path.clone());
            let critical = details.as_ref().is_some_and(|d| d.critical);
            processes.push(Process {
                pid: entry.pid,
                parent_pid: entry.parent_pid,
                group_key: group_key(&entry.name, path.as_deref()),
                protection: protection(entry.pid, &entry.name, critical, own_pid),
                name: entry.name,
                path,
                cpu_percent: rates.map(|r| r.cpu_percent),
                ram_bytes: details.as_ref().and_then(|d| d.ram_bytes),
                read_bytes_per_sec: rates.map(|r| r.read),
                write_bytes_per_sec: rates.map(|r| r.write),
            });
        }

        let (ram_used_bytes, ram_total_bytes) = memory_status();
        let totals = SystemTotals {
            cpu_percent: system_cpu,
            ram_used_bytes,
            ram_total_bytes,
            read_bytes_per_sec: elapsed
                .map(|_| processes.iter().filter_map(|p| p.read_bytes_per_sec).sum()),
            write_bytes_per_sec: elapsed
                .map(|_| processes.iter().filter_map(|p| p.write_bytes_per_sec).sum()),
            logical_cpus,
        };
        let groups = self.group(processes);

        self.previous = Some(Previous {
            at: now,
            system,
            processes: counters,
        });
        Snapshot { groups, totals }
    }

    fn group(&mut self, processes: Vec<Process>) -> Vec<ProcessGroup> {
        let mut by_key: HashMap<String, Vec<Process>> = HashMap::new();
        for process in processes {
            by_key
                .entry(process.group_key.clone())
                .or_default()
                .push(process);
        }
        by_key
            .into_iter()
            .map(|(key, mut processes)| {
                processes.sort_by_key(|p| p.pid);
                let path = processes.iter().find_map(|p| p.path.clone());
                let name = path
                    .as_ref()
                    .and_then(|p| self.description(p))
                    .unwrap_or_else(|| processes[0].name.clone());
                let sum = |f: fn(&Process) -> Option<u64>| -> Option<u64> {
                    let values: Vec<u64> = processes.iter().filter_map(f).collect();
                    (!values.is_empty()).then(|| values.iter().sum())
                };
                let cpu: Vec<f64> = processes.iter().filter_map(|p| p.cpu_percent).collect();
                ProcessGroup {
                    cpu_percent: (!cpu.is_empty()).then(|| cpu.iter().sum()),
                    ram_bytes: sum(|p| p.ram_bytes),
                    read_bytes_per_sec: sum(|p| p.read_bytes_per_sec),
                    write_bytes_per_sec: sum(|p| p.write_bytes_per_sec),
                    key,
                    name,
                    path,
                    processes,
                }
            })
            .collect()
    }

    /// The file description of an executable, read once per path.
    fn description(&mut self, path: &Path) -> Option<String> {
        self.descriptions
            .entry(path.to_path_buf())
            .or_insert_with(|| VersionInfo::read(path)?.field("FileDescription"))
            .clone()
    }
}

#[derive(Clone, Copy)]
struct Rates {
    cpu_percent: f64,
    read: u64,
    write: u64,
}

fn rates(before: &Counters, after: &Counters, seconds: f64, logical_cpus: u32) -> Rates {
    // CPU times are in 100-nanosecond units.
    let cpu_seconds = after.cpu_time.saturating_sub(before.cpu_time) as f64 / 1e7;
    let per_second = |b: u64, a: u64| (a.saturating_sub(b) as f64 / seconds).round() as u64;
    Rates {
        cpu_percent: (cpu_seconds / seconds / f64::from(logical_cpus.max(1)) * 100.0)
            .clamp(0.0, 100.0),
        read: per_second(before.read_bytes, after.read_bytes),
        write: per_second(before.write_bytes, after.write_bytes),
    }
}

fn group_key(name: &str, path: Option<&Path>) -> String {
    match path {
        Some(path) => path.to_string_lossy().to_lowercase(),
        None => name.to_lowercase(),
    }
}

fn protection(pid: u32, name: &str, critical: bool, own_pid: u32) -> Option<Protection> {
    if pid == own_pid {
        return Some(Protection::OwnProcess);
    }
    // Pid 0 is the idle process and pid 4 the kernel ("System").
    let named_critical = CRITICAL_NAMES.iter().any(|c| c.eq_ignore_ascii_case(name));
    (pid == 0 || pid == 4 || named_critical || critical).then_some(Protection::Critical)
}

struct Entry {
    pid: u32,
    parent_pid: u32,
    name: String,
}

/// Every running process, from a ToolHelp snapshot: this works for every
/// process, including those that can't be opened.
fn process_entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    // SAFETY: a process snapshot needs no pid.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return entries;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `entry.dwSize` is set as the API requires.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        entries.push(Entry {
            pid: entry.th32ProcessID,
            parent_pid: entry.th32ParentProcessID,
            name: from_wide(&entry.szExeFile),
        });
        // SAFETY: same snapshot and entry as above.
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: the snapshot handle is owned here.
    let _ = unsafe { CloseHandle(snapshot) };
    entries
}

struct Details {
    creation_time: u64,
    counters: Counters,
    ram_bytes: Option<u64>,
    path: Option<PathBuf>,
    critical: bool,
}

/// A process handle closed on drop.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by this module and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// What can be read about a process; `None` when it can't be opened (other
/// users' and protected processes).
fn read_details(pid: u32) -> Option<Details> {
    if pid == 0 {
        return None;
    }
    // SAFETY: plain query access; failure is handled.
    let handle =
        OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
    let h = handle.0;

    let (mut creation, mut exit, mut kernel, mut user) = Default::default();
    // SAFETY: out-params are valid FILETIMEs.
    unsafe { GetProcessTimes(h, &mut creation, &mut exit, &mut kernel, &mut user) }.ok()?;

    let mut io = IO_COUNTERS::default();
    // SAFETY: `io` is a valid out-param.
    let has_io = unsafe { GetProcessIoCounters(h, &mut io) }.is_ok();

    let mut memory = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        ..Default::default()
    };
    // SAFETY: `cb` matches the size of the struct passed.
    let ram_bytes = unsafe {
        GetProcessMemoryInfo(
            h,
            (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>(),
            memory.cb,
        )
    }
    .ok()
    .map(|()| memory.PrivateWorkingSetSize as u64);

    let mut buffer = vec![0u16; 32_768];
    let mut len = buffer.len() as u32;
    // SAFETY: `len` is the buffer's size in characters.
    let path = unsafe {
        QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
    }
    .ok()
    .map(|()| PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])));

    let mut critical = windows::core::BOOL(0);
    // SAFETY: `critical` is a valid out-param.
    let critical = unsafe { IsProcessCritical(h, &mut critical) }.is_ok() && critical.as_bool();

    Some(Details {
        creation_time: filetime(&creation),
        counters: Counters {
            cpu_time: filetime(&kernel) + filetime(&user),
            read_bytes: if has_io { io.ReadTransferCount } else { 0 },
            write_bytes: if has_io { io.WriteTransferCount } else { 0 },
        },
        ram_bytes,
        path,
        critical,
    })
}

fn filetime(time: &FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

fn system_times() -> Option<SystemTimes> {
    let (mut idle, mut kernel, mut user) = Default::default();
    // SAFETY: out-params are valid FILETIMEs.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    // Kernel time includes idle time.
    Some(SystemTimes {
        idle: filetime(&idle),
        busy_and_idle: filetime(&kernel) + filetime(&user),
    })
}

/// Used and total physical memory, in bytes.
fn memory_status() -> (u64, u64) {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `dwLength` is set as the API requires.
    if unsafe { GlobalMemoryStatusEx(&mut status) }.is_err() {
        return (0, 0);
    }
    (
        status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
        status.ullTotalPhys,
    )
}

fn logical_cpus() -> u32 {
    let mut info = SYSTEM_INFO::default();
    // SAFETY: `info` is a valid out-param.
    unsafe { GetSystemInfo(&mut info) };
    info.dwNumberOfProcessors.max(1)
}

/// Why a process was not ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndTaskError {
    /// Refused without trying: ending it is blocked.
    Refused(Protection),
    /// Windows denied it, usually because the process runs as administrator
    /// or as another user.
    AccessDenied,
    /// The process is no longer running.
    AlreadyExited,
    Failed(String),
}

impl std::fmt::Display for EndTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(Protection::Critical) => f.write_str("critical Windows process"),
            Self::Refused(Protection::OwnProcess) => f.write_str("Sizewise's own process"),
            Self::AccessDenied => f.write_str("access denied"),
            Self::AlreadyExited => f.write_str("the process already exited"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for EndTaskError {}

/// Ends the process `pid` and waits briefly for it to go away. Critical
/// Windows processes and Sizewise itself are refused before anything is
/// attempted. Unsaved work in the process is lost; this can't be undone.
pub fn end_process(pid: u32) -> Result<(), EndTaskError> {
    let entry = process_entries()
        .into_iter()
        .find(|e| e.pid == pid)
        .ok_or(EndTaskError::AlreadyExited)?;
    let critical = read_details(pid).is_some_and(|d| d.critical);
    if let Some(protection) = protection(pid, &entry.name, critical, std::process::id()) {
        return Err(EndTaskError::Refused(protection));
    }

    let access = PROCESS_TERMINATE | PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION;
    // SAFETY: failure is handled; the handle is closed by `OwnedHandle`.
    let handle = OwnedHandle(unsafe { OpenProcess(access, false, pid) }.map_err(|e| {
        if e.code() == ERROR_ACCESS_DENIED.to_hresult() {
            EndTaskError::AccessDenied
        } else if e.code() == ERROR_INVALID_PARAMETER.to_hresult() {
            EndTaskError::AlreadyExited
        } else {
            EndTaskError::Failed(e.message())
        }
    })?);

    let mut exit_code = 0u32;
    // SAFETY: `exit_code` is a valid out-param.
    if unsafe { GetExitCodeProcess(handle.0, &mut exit_code) }.is_ok()
        && exit_code != STILL_ACTIVE.0 as u32
    {
        return Err(EndTaskError::AlreadyExited);
    }

    // SAFETY: the handle has terminate access.
    unsafe { TerminateProcess(handle.0, 1) }.map_err(|e| {
        if e.code() == ERROR_ACCESS_DENIED.to_hresult() {
            EndTaskError::AccessDenied
        } else {
            EndTaskError::Failed(e.message())
        }
    })?;
    // SAFETY: the handle has synchronize access.
    if unsafe { WaitForSingleObject(handle.0, 5_000) } != WAIT_OBJECT_0 {
        return Err(EndTaskError::Failed(
            "the process did not exit in time".to_string(),
        ));
    }
    Ok(())
}
