use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Lock,
    Unlock { master_password: String },
    AddEntry { service: String, user: String, password: String },
    ListEntries,
    GetEntry { service: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Error(String),
}
