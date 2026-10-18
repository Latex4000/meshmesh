pub mod codec;
pub mod error;
pub mod format;
pub mod node;
pub mod state;

use crate::error::Error;
use crate::node::Inner;

pub use iroh::{EndpointAddr, EndpointId, SecretKey};
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use std::str::FromStr;
use std::sync::Arc;

pub const ALPN: &[u8] = b"meshmesh/2";

pub fn parse_ticket(s: &str) -> Result<EndpointAddr, Error> {
    Ok(EndpointTicket::from_str(s.trim())?.into())
}
#[derive(Debug, Clone)]
pub struct MeshMeshProtocol {
    inner: Arc<Inner>,
}
impl iroh::protocol::ProtocolHandler for MeshMeshProtocol {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        let from = conn.remote_id();
        info!("accepted connection from {from}");

        //self.inner.conns.lock().unwrap().insert(from, conn.clone());

        while let Ok((send, recv)) = conn.accept_bi().await {
            let inner = self.inner.clone();

            tokio::spawn(async move {
                if let Err(e) = inner.handle_stream(from, send, recv).await {
                    log::warn!("stream from {}: {e}", from.fmt_short());
                }
            });
        }

        Ok(())
    }
}
