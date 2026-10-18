use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use iroh::{EndpointAddr, EndpointId};
use rand::Rng;
use serde::{Deserialize, Serialize};
#[derive(Debug, Default)]
pub(crate) struct State {
    pub(crate) peers: HashMap<EndpointId, PeerInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub id: EndpointId,
    pub addr: Option<EndpointAddr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer {
    pub id: u8,
    pub ticket: String,
    pub rooms: [u8; 5],
    pub peers: HashMap<u8, PeerInfo>,
}

impl Peer {
    pub fn new(ticket: String) -> Self {
        let mut rng = rand::thread_rng();
        let id = rng.gen_range(0..255);

        Self {
            id,
            ticket,
            rooms: [0, 0, 0, 0, 0],
            peers: HashMap::new(),
        }
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "ID: {}\nTicket: {}", self.id, self.ticket)
    }
}

#[derive(Debug)]
pub struct Room {
    pub id: u8,
    pub members: [u8; 5],
}
