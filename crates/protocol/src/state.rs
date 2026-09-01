use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientWindow {
    Lobby,
    Direct(u8),
    Room(u8),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer {
    pub id: u8,
    pub ticket: String,
    pub rooms: [u8; 5],
    pub window: ClientWindow,
    pub peers: Vec<Peer>,
}

#[derive(Debug)]
pub struct Room {
    pub id: u8,
    pub members: [u8; 5],
}

impl Peer {
    pub fn new(ticket: String) -> Self {
        let mut rng = rand::thread_rng();
        let id = rng.gen_range(0..255);

        Self {
            id,
            ticket,
            rooms: [0, 0, 0, 0, 0],
            window: ClientWindow::Lobby,
            peers: Vec::new(),
        }
    }
}
