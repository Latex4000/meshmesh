use anyhow::{Result, anyhow};
use iroh::{Endpoint, endpoint::presets};
use iroh_ping::Ping;
use iroh_tickets::{Ticket, endpoint::EndpointTicket};
use std::{env::Args, iter::Skip};

pub async fn run_sender(args: &mut Skip<Args>) -> Result<()> {
    let ticket_str = args
        .next()
        .ok_or_else(|| anyhow!("expected ticket as the second argument"))?;
    let ticket = EndpointTicket::decode_string(&ticket_str)
        .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;

    let endpoint = Endpoint::bind(presets::N0).await?;
    let rtt = Ping::new()
        .ping(&endpoint, ticket.endpoint_addr().clone())
        .await?;
    println!("ping took: {:?} to complete", rtt);
    endpoint.close().await;
    Ok(())
}
