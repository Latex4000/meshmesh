use std::println;

use anyhow::anyhow;
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

pub const ALPN: &[u8] = b"meshmesh/1";

#[derive(Debug, Clone)]
pub struct Protocol;

impl iroh::protocol::ProtocolHandler for Protocol {
    /// The `accept` method is called for each incoming connection for our ALPN.
    ///
    /// The returned future runs on a newly spawned tokio task, so it can run as long as
    /// the connection lasts without blocking other connections.
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // We can get the remote's endpoint id from the connection.
        let endpoint_id = connection.remote_id();
        println!("accepted connection from {endpoint_id}");

        // Our protocol is a simple request-response protocol, so we expect the
        // connecting peer to open a single bi-directional stream.
        let (mut send, mut recv) = connection.accept_bi().await?;

        // Echo any bytes received back directly.
        // This will keep copying until the sender signals the end of data on the stream.
        let bytes_sent = tokio::io::copy(&mut recv, &mut send).await?;
        println!("Copied over {bytes_sent} byte(s)");

        // By calling `finish` on the send stream we signal that we will not send anything
        // further, which makes the receive stream on the other end terminate.
        send.finish()?;

        // Wait until the remote closes the connection, which it does once it
        // received the response.
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

    // Scan for ticket
    let mut connecting_ticket = String::new();
    println!("Provide a ticket to connect to, or send your ticket to someone else");
    std::io::stdin()
        .read_line(&mut connecting_ticket)
        .unwrap_or_default();

    // Open a connection to the accepting endpoint
    let connecting_ticket = connecting_ticket.trim();
    let ticket = EndpointTicket::decode_string(connecting_ticket)
        .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;
    let conn = endpoint
        .connect(ticket.endpoint_addr().clone(), crate::ALPN)
        .await?;

    println!("Connected.");

    // Open a bidirectional QUIC stream
    let (mut send, mut recv) = conn.open_bi().await?;

    // Send some data to be echoed
    send.write_all(b"Hello, world!").await?;

    // Signal the end of data for this particular stream
    send.finish()?;

    // Receive the echo, but limit reading up to maximum 1000 bytes
    let response = recv.read_to_end(1000).await?;
    assert_eq!(&response, b"Hello, world!");

    // Explicitly close the whole connection.
    conn.close(0u32.into(), b"bye!");

    // The above call only queues a close message to be sent (see how it's not async!).
    // We need to actually call this to make sure this message is sent out.
    tokio::signal::ctrl_c().await?;
    router.shutdown().await?;
    endpoint.close().await;

    // If we don't call this, but continue using the endpoint, we then the queued
    // close call will eventually be picked up and sent.
    // But always try to wait for endpoint.close().await to go through before dropping
    // the endpoint to ensure any queued messages are sent through and connections are
    // closed gracefully.
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
