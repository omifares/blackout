use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Lock,
    Unlock { master_password: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Error(String),
}
