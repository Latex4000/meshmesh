use serde::{Deserialize, Serialize};

use crate::state::Peer;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    GetDiscover(String),
    Ping,
    Direct(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong(),
    Discover(Peer),
    Err(String),
    ACK,
}
