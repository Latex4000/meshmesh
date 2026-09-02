use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::PeerInfo;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    GetDiscover(PeerInfo),
    Ping,
    Direct(String),
    Disconnect(PeerInfo),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong(DateTime<Utc>),
    Discover(PeerInfo),
    Err(String),
    ACK,
}
