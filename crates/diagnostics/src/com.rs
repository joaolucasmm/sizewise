//! Running COM work (Task Scheduler, shell links, WMI) on a thread of its own.
//!
//! The caller's thread may already use COM in another mode (the desktop app's
//! main thread is single-threaded for the WebView), so every COM call made by
//! this crate happens on a short-lived thread it initializes itself.

use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

pub fn on_com_thread<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                // SAFETY: initializes COM for this new thread only; balanced below.
                let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
                let result = work();
                if initialized {
                    // SAFETY: matches the successful CoInitializeEx above.
                    unsafe { CoUninitialize() };
                }
                result
            })
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}
