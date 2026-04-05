use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Lock,
    Unlock {
        master_password: String,
    },
    AddEntry {
        service: String,
        user: String,
        password: String,
    },
    ListEntries,
    GetEntry {
        service: String,
    },
    GetEntryById {
        uuid: uuid::Uuid,
    },
    DeleteEntry {
        uuid: uuid::Uuid,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Error(String),
}
