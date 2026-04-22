use chrono::{DateTime, Local};
use uuid::Uuid;

use blackout_core::vault::Entry;

pub trait EntryView {
    fn _id(&self) -> &uuid::Uuid;
    fn service(&self) -> &str;
    fn username(&self) -> &str;
    fn updated_at(&self) -> DateTime<Local>;
}

#[derive(Debug, Clone)]
pub struct ListEntryView(pub Entry);

impl EntryView for ListEntryView {
    fn _id(&self) -> &Uuid {
        &self.0.id
    }
    fn service(&self) -> &str {
        &self.0.service
    }
    fn username(&self) -> &str {
        &self.0.username
    }
    fn updated_at(&self) -> DateTime<Local> {
        self.0.updated_at
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetailEntryView {
    pub entry: Entry,
    pub show_password: bool,
}
