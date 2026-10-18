use chrono::{DateTime, Utc};
use iroh::EndpointAddr;
use serde::{Deserialize, Serialize};

use crate::state::PeerInfo;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    GetDiscover(EndpointAddr),
    Ping,
    Direct(String),
    Leaving,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong(DateTime<Utc>),
    Peers(Vec<PeerInfo>),
    Err(String),
    ACK,
}
