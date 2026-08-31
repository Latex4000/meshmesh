use std::println;

use anyhow::anyhow;
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};
use tokio::io::AsyncReadExt;

pub const ALPN: &[u8] = b"meshmesh/1";

#[derive(Debug, Clone)]
pub struct Protocol;

impl iroh::protocol::ProtocolHandler for Protocol {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        let endpoint_id = conn.remote_id();
        println!("accepted connection from {endpoint_id}");

        let mut recv = conn.accept_uni().await?;
        loop {
            let mut buffer = String::new();
            if let Err(e) = recv.read_to_string(&mut buffer).await {
                println!("Error: {}", e);
                continue;
            };

            if buffer.is_empty() {
                continue;
            }

            if buffer == "0" {
                conn.close(0u32.into(), b"bye!");
                break;
            }

            println!("{buffer}");
        }

        Ok(())
    }
}

pub async fn connect() -> anyhow::Result<()> {
    let endpoint = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await?;
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(crate::ALPN, crate::Protocol)
        .spawn();
    endpoint.online().await;

    let ticket = EndpointTicket::new(endpoint.addr());
    println!("{ticket}");

    let mut connecting_ticket = String::new();
    println!("Provide a ticket to connect to, or send your ticket to someone else");
    std::io::stdin()
        .read_line(&mut connecting_ticket)
        .unwrap_or_default();

    let connecting_ticket = connecting_ticket.trim();
    let ticket = EndpointTicket::decode_string(connecting_ticket)
        .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;
    let conn = endpoint
        .connect(ticket.endpoint_addr().clone(), crate::ALPN)
        .await?;

    println!("Connected.");

    // Send some data
    let mut send = conn.open_uni().await?;
    loop {
        let mut send_text = String::new();
        std::io::stdin()
            .read_line(&mut send_text)
            .unwrap_or_default();
        send.write_all(&send_text.clone().into_bytes()).await?;
        send.finish()?;

        if send_text == "0" {
            break;
        }
    }

    conn.closed().await;

    tokio::signal::ctrl_c().await?;
    router.shutdown().await?;
    endpoint.close().await;

    Ok(())
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
