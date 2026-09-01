use serde::{Deserialize, Serialize};

use crate::state::PeerInfo;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    GetDiscover(PeerInfo),
    Ping,
    Direct(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong(),
    Discover(PeerInfo),
    Err(String),
    ACK,
}
