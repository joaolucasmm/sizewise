use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use scanner::ScanResult;

/// Only one Scan at a time: starting a new one cancels the running one and
/// discards the previous result.
#[derive(Default)]
pub struct AppState(Mutex<Slot>);

#[derive(Default)]
struct Slot {
    scan_id: u64,
    cancel: Arc<AtomicBool>,
    result: Option<Arc<ScanResult>>,
}

impl AppState {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Cancels any running Scan, drops the last result and returns the id
    /// and cancellation flag of the new Scan.
    pub fn begin(&self) -> (u64, Arc<AtomicBool>) {
        let mut slot = self.lock();
        slot.cancel.store(true, Ordering::Relaxed);
        slot.scan_id += 1;
        slot.cancel = Arc::new(AtomicBool::new(false));
        slot.result = None;
        (slot.scan_id, slot.cancel.clone())
    }

    pub fn is_current(&self, scan_id: u64) -> bool {
        self.lock().scan_id == scan_id
    }

    /// Stores the result if `scan_id` is still the latest Scan.
    pub fn finish(&self, scan_id: u64, result: Arc<ScanResult>) -> bool {
        let mut slot = self.lock();
        if slot.scan_id != scan_id {
            return false;
        }
        slot.result = Some(result);
        true
    }

    pub fn cancel(&self) {
        self.lock().cancel.store(true, Ordering::Relaxed);
    }

    pub fn result(&self) -> Result<Arc<ScanResult>, String> {
        self.lock()
            .result
            .clone()
            .ok_or_else(|| "no scan result available".to_string())
    }
}
