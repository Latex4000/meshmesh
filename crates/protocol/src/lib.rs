pub mod codec;
pub mod format;
pub mod state;

use crate::codec::{codec, open_stream, read_msg, write_msg};
use crate::format::{Request, Response};
use crate::state::Peer;
use anyhow::{Context, bail};
use chrono::Utc;
use futures::SinkExt;
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use std::sync::{Mutex, OnceLock};
use tokio_util::codec::{FramedRead, FramedWrite};

pub const ALPN: &[u8] = b"meshmesh/1";

static ROUTER: tokio::sync::OnceCell<iroh::protocol::Router> = tokio::sync::OnceCell::const_new();
static ENDPOINT: tokio::sync::OnceCell<iroh::Endpoint> = tokio::sync::OnceCell::const_new();
static CLIENT_CTX: OnceLock<Mutex<Peer>> = OnceLock::new();
pub fn use_ctx<T>(f: impl FnOnce(&mut Peer) -> T) -> T {
    let mutex = CLIENT_CTX.get().expect("init() has not been called");
    let mut ctx = mutex.lock().unwrap();
    f(&mut ctx)
}

#[derive(Debug, Clone)]
pub struct MeshMeshProtocol;

impl iroh::protocol::ProtocolHandler for MeshMeshProtocol {
    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        let endpoint_id = conn.remote_id();
        info!("accepted connection from {endpoint_id}");

        while let Ok((send, recv)) = conn.accept_bi().await {
            let mut tx = FramedWrite::new(send, codec());
            let mut rx = FramedRead::new(recv, codec());

            let Some(req) = read_msg::<Request>(&mut rx).await.unwrap() else {
                return Ok(()); // peer opened a stream and closed it without asking anything
            };

            let mut responses = Vec::new();
            {
                use_ctx(|ctx| {
                    match req {
                        Request::GetDiscover(peer_info) => {
                            ctx.peers.insert(peer_info.id, peer_info.clone());
                            responses.push(Response::Discover(ctx.get_info()));
                            for peer in ctx.peers.iter().filter(|(k, _v)| **k != peer_info.id) {
                                responses.push(Response::Discover(peer.1.clone()));
                            }
                        }
                        Request::Ping => responses.push(Response::Pong(Utc::now())),
                        Request::Direct(data) => {
                            info!("got dm -> {data}");
                            responses.push(Response::ACK);
                        }
                    };
                })
            }
            for x in responses {
                let _ = write_msg(&mut tx, &x).await;
            }

            tx.close().await?;
        }

        Ok(())
    }
}

pub async fn init() -> anyhow::Result<()> {
    let endpoint = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await?;
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(crate::ALPN, crate::MeshMeshProtocol)
        .spawn();
    endpoint.online().await;
    ROUTER.set(router).ok();

    let ticket = EndpointTicket::new(endpoint.addr());
    ENDPOINT.set(endpoint).ok();
    let peer = Peer::new(ticket.to_string());
    CLIENT_CTX.set(Mutex::new(peer)).ok();
    Ok(())
}

impl Peer {
    pub async fn send_to(recipient: u8, data: String) -> anyhow::Result<()> {
        let ticket = use_ctx(|ctx| {
            let peer = ctx.peers.get(&recipient)?;
            Some(peer.ticket.clone())
        })
        .context("Peer not found")?;

        let (mut tx, mut rx) = open_stream(&ticket).await?;

        write_msg(&mut tx, &Request::Direct(data)).await?;

        tx.close().await?;

        match read_msg::<Response>(&mut rx).await? {
            Some(Response::ACK) => Ok(()),
            other => bail!("{other:?}"),
        }
    }
    pub async fn discover(ticket: &str) -> anyhow::Result<()> {
        let self_info = use_ctx(|ctx| ctx.get_info());
        if self_info.ticket == ticket {
            bail!("Can't connect to yourself");
        }

        let (mut tx, mut rx) = open_stream(ticket).await?;

        write_msg(&mut tx, &Request::GetDiscover(self_info)).await?;

        while let Some(Response::Discover(peer_info)) = read_msg::<Response>(&mut rx).await? {
            use_ctx(|ctx| ctx.peers.insert(peer_info.id, peer_info));
        }
        tx.close().await?;
        Ok(())
    }

    pub async fn ping(ticket: &str) -> anyhow::Result<()> {
        let self_info = use_ctx(|ctx| ctx.get_info());
        if self_info.ticket == ticket {
            bail!("Can't connect to yourself");
        }

        let ticket = match ticket.parse::<u8>() {
            Ok(peer_id) => use_ctx(|ctx| {
                let peer = ctx.peers.get(&peer_id)?;
                Some(peer.ticket.clone())
            })
            .context("Peer not found")?,
            Err(_) => ticket.to_string(),
        };
        let (mut tx, mut rx) = open_stream(&ticket).await?;

        let init_time = Utc::now();
        write_msg(&mut tx, &Request::Ping).await?;

        match read_msg::<Response>(&mut rx).await? {
            Some(Response::Pong(final_time)) => {
                println!(
                    "Pong: {}ms",
                    final_time
                        .signed_duration_since(init_time)
                        .num_milliseconds()
                )
            }
            _ => eprintln!("closed without replying"),
        }
        tx.close().await?;
        Ok(())
    }
}
