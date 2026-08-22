use anyhow::Result;
use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_ping::Ping;
use iroh_tickets::endpoint::EndpointTicket;

pub async fn run_receiver() -> Result<()> {
    let endpoint = Endpoint::bind(presets::N0).await?;
    endpoint.online().await;

    let ticket = EndpointTicket::new(endpoint.addr());
    println!("{ticket}");

    let _router = Router::builder(endpoint)
        .accept(iroh_ping::ALPN, Ping::new())
        .spawn();

    tokio::signal::ctrl_c().await?;
    _router.shutdown().await?;
    Ok(())
}
