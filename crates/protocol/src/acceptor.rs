use iroh_tickets::endpoint::EndpointTicket;

pub async fn accept() -> anyhow::Result<()> {
    let endpoint = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await?;
    endpoint.online().await;

    let ticket = EndpointTicket::new(endpoint.addr());
    println!("{ticket}");

    let router = iroh::protocol::Router::builder(endpoint)
        .accept(crate::ALPN, crate::Protocol)
        .spawn();

    tokio::signal::ctrl_c().await?;
    router.shutdown().await?;
    Ok(())
}
