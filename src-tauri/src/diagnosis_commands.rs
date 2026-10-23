//! Commands of the Diagnoses sections. Reading Diagnoses can take a moment
//! (registry, Task Scheduler, WMI), so those commands run off the UI thread.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use diagnostics::{InstalledProgram, ProcessSampler};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::diagnosis_dto::{
    disks_dto, Availability, EndTaskFailureDto, OverviewDto, PhysicalDiskDto, ProgramDto,
    SnapshotDto, StartupDurationDto, StartupItemDto, UninstallFailureDto, UninstallFinishedDto,
    VolumeDto,
};
use crate::state::SamplerState;

/// Time between two process snapshots.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(1500);

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_overview() -> Result<OverviewDto, String> {
    blocking(|| diagnostics::overview().into()).await
}

#[tauri::command]
pub async fn list_startup_items() -> Result<Vec<StartupItemDto>, String> {
    blocking(|| {
        diagnostics::list_startup_items()
            .into_iter()
            .map(Into::into)
            .collect()
    })
    .await
}

#[tauri::command]
pub async fn get_startup_durations() -> Result<Availability<Vec<StartupDurationDto>>, String> {
    blocking(|| {
        Availability::new(&diagnostics::startup_durations(10), |durations| {
            durations.iter().map(Into::into).collect()
        })
    })
    .await
}

/// Starts emitting `process-snapshot` events until `stop_process_sampling`.
/// Does nothing if sampling is already running.
#[tauri::command]
pub fn start_process_sampling(app: AppHandle, sampler: State<'_, SamplerState>) {
    let Some(stop) = sampler.start() else {
        return;
    };
    std::thread::spawn(move || {
        let mut sampler = ProcessSampler::new();
        // The first snapshot has no rates; the second comes sooner so the
        // screen fills in quickly.
        let mut wait = Duration::from_millis(500);
        loop {
            let snapshot = sampler.sample();
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let _ = app.emit("process-snapshot", SnapshotDto::from(&snapshot));
            let deadline = std::time::Instant::now() + wait;
            while std::time::Instant::now() < deadline {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            wait = SAMPLE_INTERVAL;
        }
    });
}

#[tauri::command]
pub fn stop_process_sampling(sampler: State<'_, SamplerState>) {
    sampler.stop();
}

#[tauri::command]
pub async fn end_process(pid: u32) -> Result<(), EndTaskFailureDto> {
    tauri::async_runtime::spawn_blocking(move || diagnostics::end_process(pid))
        .await
        .map_err(|e| EndTaskFailureDto::from(diagnostics::EndTaskError::Failed(e.to_string())))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_installed_programs() -> Result<Vec<ProgramDto>, String> {
    blocking(|| {
        diagnostics::list_installed_programs()
            .into_iter()
            .map(Into::into)
            .collect()
    })
    .await
}

/// The program is looked up again by id, so the UI never passes a command
/// line or a path to run.
fn find_program(id: &str) -> Option<InstalledProgram> {
    diagnostics::list_installed_programs()
        .into_iter()
        .find(|p| p.id == id)
}

/// Measures a program's install folder; returns its size in bytes.
#[tauri::command]
pub async fn measure_program(id: String) -> Result<u64, String> {
    blocking(move || {
        let program = find_program(&id).ok_or("the program is no longer installed")?;
        diagnostics::measure_install_folder(&program).map_err(|e| e.to_string())
    })
    .await?
}

/// Starts the program's own uninstaller. When it exits, an
/// `uninstall-finished` event carries the program id.
#[tauri::command]
pub async fn start_uninstall(app: AppHandle, id: String) -> Result<(), UninstallFailureDto> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        let program = find_program(&id).ok_or_else(UninstallFailureDto::not_found)?;
        let uninstaller = diagnostics::start_uninstall(&program)?;
        std::thread::spawn(move || {
            let exit_code = uninstaller.wait();
            let _ = app.emit("uninstall-finished", UninstallFinishedDto { id, exit_code });
        });
        Ok(())
    })
    .await;
    match result {
        Ok(outcome) => outcome,
        Err(error) => Err(diagnostics::UninstallError::Failed(error.to_string()).into()),
    }
}

#[tauri::command]
pub async fn list_physical_disks() -> Result<Availability<Vec<PhysicalDiskDto>>, String> {
    blocking(|| disks_dto(&diagnostics::list_physical_disks())).await
}

#[tauri::command]
pub fn list_volumes() -> Vec<VolumeDto> {
    diagnostics::list_volumes()
        .into_iter()
        .map(Into::into)
        .collect()
}

/// Opens Explorer with a file selected (a program, a Startup item's target).
#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(PathBuf::from(path))
        .map_err(|e| e.to_string())
}

/// Opens a folder in Explorer (a program's install folder). Only folders:
/// this never runs a file.
#[tauri::command]
pub fn open_folder(app: AppHandle, path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return Err("not a folder".to_string());
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}
