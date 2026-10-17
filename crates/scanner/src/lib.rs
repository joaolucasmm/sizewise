//! Disk usage scanning core for Sizewise.
//!
//! This crate has no Tauri dependency: it scans a Target, keeps the indexed
//! result tree in memory and answers queries on it. The desktop app is a thin
//! layer that forwards commands and events between the UI and this crate.

mod drives;
mod result;
mod walk;

pub use drives::{list_drives, Drive};
pub use result::{Node, NodeId, NodeKind, ScanResult, ScanState};
pub use walk::{scan, Progress, ScanError, ScanOptions};
