use serde::{Deserialize, Serialize};
use serde_cbor::{from_slice, to_vec};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Result, Write};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum EventType {
    VaultCreated,
    EntryCreated,
    EntryUpdated,
    EntryDeleted,
    EntryAccessed,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    pub event_id: Uuid,
    pub timestamp: i64,
    pub event_type: EventType,
    pub entry_id: Option<Uuid>,
    pub payload: Vec<u8>,
}

pub fn append_event(event: &Event) -> Result<()> {
    let encoded = to_vec(event).unwrap();
    let len = encoded.len() as u32;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("events.log")?;

    file.write_all(&len.to_le_bytes())?;
    file.write_all(&encoded)?;
    Ok(())
}

pub fn load_events() -> Result<Vec<Event>> {
    let mut file = File::open("events.log")?;
    let mut events = Vec::new();

    loop {
        let mut len_buf = [0u8; 4];
        if file.read_exact(&mut len_buf).is_err() {
            break;
        }

        let len = u32::from_le_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf)?;

        let event = from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        events.push(event);
    }

    Ok(events)
}
