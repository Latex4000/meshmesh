use anyhow::Result;
use protocol::{acceptor::accept, connector::connect};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("{}", protocol::add(1, 2));
    let mut args = env::args().skip(1);
    let role = args.next().unwrap_or_default();

    match role.as_str() {
        "sender" => connect(&mut args).await,
        _ => accept().await,
    }
}
