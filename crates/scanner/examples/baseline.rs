//! Times a Standard scan of a Target, for the README baseline.
//!
//! ```text
//! cargo run --release -p scanner --example baseline -- C:\
//! ```

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use scanner::{scan, ScanOptions};

fn main() {
    let target = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "C:\\".into()));
    let result = scan(
        &target,
        &ScanOptions::default(),
        &|_| {},
        &AtomicBool::new(false),
    )
    .unwrap_or_else(|e| panic!("scan failed: {e}"));

    println!("target:      {}", target.display());
    println!("duration:    {:.1} s", result.duration().as_secs_f64());
    println!(
        "total size:  {:.1} GB",
        result.total_size() as f64 / (1u64 << 30) as f64
    );
    println!("files:       {}", result.total_files());
    println!("folders:     {}", result.total_folders());
    println!("unreadable:  {}", result.unreadable().len());
    println!(
        "threads:     {}",
        std::thread::available_parallelism().map_or(0, |n| n.get())
    );
}
