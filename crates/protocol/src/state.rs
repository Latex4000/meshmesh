use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientWindow {
    Lobby,
    Direct(Uuid),
    Room(u8),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub id: Uuid,
    pub ticket: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer {
    pub id: Uuid,
    pub ticket: String,
    pub rooms: [u8; 5],
    pub window: ClientWindow,
    pub peers: HashMap<Uuid, PeerInfo>,
}

impl Peer {
    pub fn new(ticket: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            ticket,
            rooms: [0, 0, 0, 0, 0],
            window: ClientWindow::Lobby,
            peers: HashMap::new(),
        }
    }

    pub fn get_info(&self) -> PeerInfo {
        PeerInfo {
            id: self.id,
            ticket: self.ticket.clone(),
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
