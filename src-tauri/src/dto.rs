//! Shapes sent to the UI. Field names are camelCase on the TypeScript side.

use scanner::{Drive, NodeId, NodeKind, Progress, ScanError, ScanResult, ScanState};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveDto {
    pub root: String,
    pub letter: String,
    pub label: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

impl From<Drive> for DriveDto {
    fn from(drive: Drive) -> Self {
        Self {
            root: drive.root.to_string_lossy().into_owned(),
            letter: drive.letter.to_string(),
            label: drive.label,
            file_system: drive.file_system,
            total_bytes: drive.total_bytes,
            free_bytes: drive.free_bytes,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum KindDto {
    Folder,
    File,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeDto {
    pub id: NodeId,
    pub name: String,
    pub kind: KindDto,
    pub size: u64,
    pub files: u64,
    pub folders: u64,
    pub modified: Option<i64>,
    pub child_count: usize,
    pub path: String,
}

impl NodeDto {
    pub fn new(result: &ScanResult, id: NodeId) -> Option<Self> {
        let node = result.node(id)?;
        Some(Self {
            id,
            name: node.name().to_string(),
            kind: match node.kind() {
                NodeKind::Folder => KindDto::Folder,
                NodeKind::File => KindDto::File,
            },
            size: node.size(),
            files: node.files(),
            folders: node.folders(),
            modified: node.modified(),
            child_count: node.child_count(),
            path: result.path(id)?.to_string_lossy().into_owned(),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenPage {
    pub items: Vec<NodeDto>,
    pub total: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum StateDto {
    Completed,
    Cancelled,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDto {
    pub scan_id: u64,
    pub target: String,
    pub root: NodeDto,
    pub total_size: u64,
    pub total_files: u64,
    pub total_folders: u64,
    pub duration_ms: u64,
    pub state: StateDto,
    pub unreadable_count: usize,
}

impl SummaryDto {
    pub fn new(scan_id: u64, result: &ScanResult) -> Self {
        Self {
            scan_id,
            target: result.target().to_string_lossy().into_owned(),
            root: NodeDto::new(result, result.root()).expect("root exists"),
            total_size: result.total_size(),
            total_files: result.total_files(),
            total_folders: result.total_folders(),
            duration_ms: result.duration().as_millis() as u64,
            state: match result.state() {
                ScanState::Completed => StateDto::Completed,
                ScanState::Cancelled => StateDto::Cancelled,
            },
            unreadable_count: result.unreadable().len(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressDto {
    pub scan_id: u64,
    pub current_folder: String,
    pub files: u64,
    pub bytes: u64,
    pub elapsed_ms: u64,
}

impl ProgressDto {
    pub fn new(scan_id: u64, progress: &Progress) -> Self {
        Self {
            scan_id,
            current_folder: progress.current_folder.to_string_lossy().into_owned(),
            files: progress.files,
            bytes: progress.bytes,
            elapsed_ms: progress.elapsed.as_millis() as u64,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FailureReason {
    NotFound,
    NotAFolder,
    Unreadable,
    Internal,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FailedDto {
    pub scan_id: u64,
    pub target: String,
    pub reason: FailureReason,
    pub message: String,
}

impl FailedDto {
    pub fn new(scan_id: u64, target: String, error: &ScanError) -> Self {
        Self {
            scan_id,
            target,
            reason: match error {
                ScanError::TargetNotFound(_) => FailureReason::NotFound,
                ScanError::NotAFolder(_) => FailureReason::NotAFolder,
                ScanError::TargetUnreadable { .. } => FailureReason::Unreadable,
                ScanError::Threads(_) => FailureReason::Internal,
            },
            message: error.to_string(),
        }
    }
}
