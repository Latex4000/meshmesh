use protocol::state::Peer;
#[derive(Debug, Clone)]
pub enum ClientState {
    Lobby,
    Direct(u8),
    Room(u8),
}
#[derive(Debug)]
pub struct Context {
    pub state: ClientState,
    pub local: Peer,
    pub peers: Vec<protocol::state::Peer>,
}
