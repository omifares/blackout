use blackout_core::vault::{Entry, VaultSnapshot};
use chrono::{DateTime, Local};

pub struct SnapshotView {
    pub version: u32,
    pub created_at: DateTime<Local>,
    pub checksum: String,
    pub reason: String,
}

pub enum SelectedItem<'a> {
    Entry(&'a Entry),
    Snapshot(&'a VaultSnapshot),
}
