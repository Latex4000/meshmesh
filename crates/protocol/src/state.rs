use rand::Rng;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer {
    pub id: u8,
    pub ticket: String,
    pub rooms: [u8; 5],
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
            id: id.into(),
            ticket,
            rooms: [0, 0, 0, 0, 0],
        }
    }
}
