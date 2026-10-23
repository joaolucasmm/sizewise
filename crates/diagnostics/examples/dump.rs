//! Prints every Diagnosis of this PC, to check the crate by eye:
//! `cargo run -p diagnostics --example dump`.

fn main() {
    println!("== Startup items");
    for item in diagnostics::list_startup_items() {
        println!(
            "{:?} | {} | {:?} | {:?} | broken: {} | {}",
            item.state,
            item.name,
            item.publisher,
            item.source,
            item.is_broken(),
            item.command
        );
    }
    println!(
        "\n== Startup durations: {:?}",
        diagnostics::startup_durations(3)
    );

    println!("\n== Installed programs");
    let mut programs = diagnostics::list_installed_programs();
    programs.sort_by_key(|p| std::cmp::Reverse(p.size_bytes));
    for program in &programs {
        println!(
            "{:>12?} | {} | {:?} | {:?} | {:?} | {}",
            program.size_bytes,
            program.name,
            program.publisher,
            program.version,
            program.install_date,
            program.id
        );
    }

    println!("\n== Disks: {:#?}", diagnostics::list_physical_disks());
    for volume in diagnostics::list_volumes() {
        println!(
            "{}: {} / {} low: {}",
            volume.drive.letter,
            volume.drive.free_bytes,
            volume.drive.total_bytes,
            volume.low_space
        );
    }

    println!("\n== Processes (top 10 by RAM, second sample)");
    let mut sampler = diagnostics::ProcessSampler::new();
    sampler.sample();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let snapshot = sampler.sample();
    let mut groups = snapshot.groups.clone();
    groups.sort_by_key(|g| std::cmp::Reverse(g.ram_bytes));
    for group in groups.iter().take(10) {
        println!(
            "{:>5.1?}% {:>12} B {:>3}x | {} | read {:?} write {:?}",
            group.cpu_percent,
            group.ram_bytes.unwrap_or(0),
            group.processes.len(),
            group.name,
            group.read_bytes_per_sec,
            group.write_bytes_per_sec
        );
    }
    println!("{:?}", snapshot.totals);
}
