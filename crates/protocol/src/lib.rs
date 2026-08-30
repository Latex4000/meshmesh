use std::println;

use anyhow::anyhow;
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

pub const ALPN: &[u8] = b"meshmesh/1";

#[derive(Debug, Clone)]
pub struct Protocol;

impl iroh::protocol::ProtocolHandler for Protocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let endpoint_id = connection.remote_id();
        println!("accepted connection from {endpoint_id}");

        let mut recv = connection.accept_uni().await?;
        let mut buf: Vec<u8> = vec![];
        match recv.read(&mut buf).await {
            Ok(_) => {}
            Err(e) => println!("Error: {}", e),
        };
        let buf_str = match str::from_utf8(&buf) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        println!("{}", buf_str);

        connection.closed().await;

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

    let mut send = conn.open_uni().await?;

    // Send some data
    send.write_all(b"Hello, world!").await?;
    send.finish()?;

    conn.close(0u32.into(), b"bye!");

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
