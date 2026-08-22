use anyhow::Result;
use std::env;

use crate::receiver::run_receiver;
use crate::sender::run_sender;

mod receiver;
mod sender;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut args = env::args().skip(1);
    let role = args.next().unwrap_or_default();

    match role.as_str() {
        "sender" => run_sender(&mut args).await,
        _ => run_receiver().await,
    }
}
