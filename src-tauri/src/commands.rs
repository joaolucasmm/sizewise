use std::path::PathBuf;
use std::sync::Arc;

use scanner::{NodeId, ScanOptions};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::dto::{ChildrenPage, DriveDto, FailedDto, NodeDto, ProgressDto, SummaryDto};
use crate::state::AppState;

/// How many Large files the results screen shows.
const TOP_FILES: usize = 100;

/// Windows display language (e.g. `pt-BR`), used for the first launch.
#[tauri::command]
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

#[tauri::command]
pub fn list_drives() -> Vec<DriveDto> {
    scanner::list_drives()
        .into_iter()
        .map(DriveDto::from)
        .collect()
}

/// Starts a Scan in the background and returns its id. Progress and the
/// outcome arrive as `scan-progress`, `scan-completed`, `scan-cancelled` and
/// `scan-failed` events carrying that id.
#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<'_, AppState>, target: String) -> u64 {
    let (scan_id, cancel) = state.begin();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let on_progress = |progress: &scanner::Progress| {
            if state.is_current(scan_id) {
                let _ = app.emit("scan-progress", ProgressDto::new(scan_id, progress));
            }
        };
        let options = ScanOptions {
            top_n: TOP_FILES,
            ..ScanOptions::default()
        };
        match scanner::scan(&PathBuf::from(&target), &options, &on_progress, &cancel) {
            Ok(result) => {
                let summary = SummaryDto::new(scan_id, &result);
                let event = match result.state() {
                    scanner::ScanState::Completed => "scan-completed",
                    scanner::ScanState::Cancelled => "scan-cancelled",
                };
                if state.finish(scan_id, Arc::new(result)) {
                    let _ = app.emit(event, summary);
                }
            }
            Err(error) => {
                if state.is_current(scan_id) {
                    let _ = app.emit("scan-failed", FailedDto::new(scan_id, target, &error));
                }
            }
        }
    });
    scan_id
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.cancel();
}

#[tauri::command]
pub fn get_summary(state: State<'_, AppState>, scan_id: u64) -> Result<SummaryDto, String> {
    let result = state.result()?;
    Ok(SummaryDto::new(scan_id, &result))
}

#[tauri::command]
pub fn get_children(
    state: State<'_, AppState>,
    id: NodeId,
    offset: usize,
    limit: usize,
) -> Result<ChildrenPage, String> {
    let result = state.result()?;
    let total = result.node(id).ok_or("unknown node")?.child_count();
    let items = result
        .children(id, offset, limit)
        .into_iter()
        .filter_map(|child| NodeDto::new(&result, child))
        .collect();
    Ok(ChildrenPage { items, total })
}

#[tauri::command]
pub fn get_top_files(state: State<'_, AppState>, limit: usize) -> Result<Vec<NodeDto>, String> {
    let result = state.result()?;
    Ok(result
        .top_files(limit)
        .iter()
        .filter_map(|&id| NodeDto::new(&result, id))
        .collect())
}

#[tauri::command]
pub fn get_unreadable(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state
        .result()?
        .unreadable()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

/// Opens Explorer with the item selected.
#[tauri::command]
pub fn reveal_in_explorer(
    app: AppHandle,
    state: State<'_, AppState>,
    id: NodeId,
) -> Result<(), String> {
    let path = state.result()?.path(id).ok_or("unknown node")?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}
