//! The headline Diagnoses at a glance. Live CPU and RAM use come from the
//! process sampler, not from here.

use crate::{
    list_physical_disks, list_startup_items, list_volumes, startup_durations, PhysicalDisk,
    StartupDuration, StartupState, Unavailable, Volume,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Overview {
    pub volumes: Vec<Volume>,
    pub enabled_startup_items: usize,
    /// The most recent startup Windows measured, if any.
    pub last_startup: Result<Option<StartupDuration>, Unavailable>,
    pub disks: Result<Vec<PhysicalDisk>, Unavailable>,
}

pub fn overview() -> Overview {
    Overview {
        volumes: list_volumes(),
        enabled_startup_items: list_startup_items()
            .iter()
            .filter(|item| item.state == StartupState::Enabled)
            .count(),
        last_startup: startup_durations(1).map(|d| d.into_iter().next()),
        disks: list_physical_disks(),
    }
}
