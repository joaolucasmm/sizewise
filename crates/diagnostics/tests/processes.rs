//! Behavior of the Resource monitor and "End task" on the real Windows.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use diagnostics::{end_process, EndTaskError, ProcessSampler, Protection};

/// A child process that idles until ended.
fn spawn_idle_child() -> Child {
    Command::new("ping")
        .args(["-n", "120", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn snapshot_contains_this_process_with_its_ram() {
    let snapshot = ProcessSampler::new().sample();
    let own = snapshot
        .processes()
        .find(|p| p.pid == std::process::id())
        .expect("own process listed");
    assert!(own.ram_bytes.unwrap_or(0) > 0);
    assert!(own.path.as_ref().is_some_and(|p| p.is_file()));
    assert_eq!(own.protection, Some(Protection::OwnProcess));
    // The first sample has no previous one to compute rates from.
    assert_eq!(own.cpu_percent, None);
    assert_eq!(snapshot.totals.cpu_percent, None);
}

#[test]
fn second_sample_reports_cpu_and_io_rates() {
    let mut sampler = ProcessSampler::new();
    sampler.sample();
    std::thread::sleep(Duration::from_millis(300));
    let snapshot = sampler.sample();

    let own = snapshot
        .processes()
        .find(|p| p.pid == std::process::id())
        .unwrap();
    let cpu = own.cpu_percent.expect("CPU rate on second sample");
    assert!((0.0..=100.0).contains(&cpu));
    assert!(own.read_bytes_per_sec.is_some());
    assert!(own.write_bytes_per_sec.is_some());

    let totals = &snapshot.totals;
    let system_cpu = totals.cpu_percent.expect("system CPU on second sample");
    assert!((0.0..=100.0).contains(&system_cpu));
    assert!(totals.ram_total_bytes > 0);
    assert!(totals.ram_used_bytes > 0 && totals.ram_used_bytes <= totals.ram_total_bytes);
    assert!(totals.logical_cpus >= 1);
    assert!(totals.read_bytes_per_sec.is_some());
}

#[test]
fn groups_sum_their_processes() {
    let snapshot = ProcessSampler::new().sample();
    assert!(!snapshot.groups.is_empty());
    for group in &snapshot.groups {
        assert!(!group.processes.is_empty());
        assert!(!group.name.is_empty());
        assert!(group.processes.iter().all(|p| p.group_key == group.key));
        let ram: Vec<u64> = group.processes.iter().filter_map(|p| p.ram_bytes).collect();
        let expected = (!ram.is_empty()).then(|| ram.iter().sum());
        assert_eq!(group.ram_bytes, expected);
    }
}

#[test]
fn processes_that_cant_be_opened_are_still_listed() {
    let snapshot = ProcessSampler::new().sample();
    // "System" (pid 4) always exists and is never fully readable.
    let system = snapshot
        .processes()
        .find(|p| p.pid == 4)
        .expect("System listed");
    assert!(!system.name.is_empty());
    assert_eq!(system.protection, Some(Protection::Critical));
}

#[test]
fn end_task_ends_a_child_and_then_reports_it_already_exited() {
    let mut child = spawn_idle_child();
    let pid = child.id();

    assert_eq!(end_process(pid), Ok(()));
    let status = child.wait().unwrap();
    assert!(!status.success());

    assert_eq!(end_process(pid), Err(EndTaskError::AlreadyExited));
}

#[test]
fn end_task_refuses_critical_processes_without_trying() {
    let snapshot = ProcessSampler::new().sample();
    let csrss = snapshot
        .processes()
        .find(|p| p.name.eq_ignore_ascii_case("csrss.exe"))
        .expect("csrss is always running");
    assert_eq!(csrss.protection, Some(Protection::Critical));
    assert_eq!(
        end_process(csrss.pid),
        Err(EndTaskError::Refused(Protection::Critical))
    );
}

#[test]
fn end_task_refuses_sizewise_itself() {
    assert_eq!(
        end_process(std::process::id()),
        Err(EndTaskError::Refused(Protection::OwnProcess))
    );
}
