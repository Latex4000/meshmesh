use anyhow::Result;
use protocol::connect;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("{}", protocol::add(1, 2));
    connect().await
}
