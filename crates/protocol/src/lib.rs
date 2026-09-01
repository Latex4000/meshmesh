pub mod codec;
pub mod format;
pub mod state;

use std::sync::{Mutex, OnceLock};

use crate::codec::codec;
use crate::format::{Request, Response};
use crate::state::Peer;
use anyhow::{anyhow, bail};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use iroh::endpoint::{RecvStream, SendStream};
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

use log::info;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
pub const ALPN: &[u8] = b"meshmesh/1";
static ROUTER: tokio::sync::OnceCell<iroh::protocol::Router> = tokio::sync::OnceCell::const_new();
static ENDPOINT: tokio::sync::OnceCell<iroh::Endpoint> = tokio::sync::OnceCell::const_new();
pub static CLIENT_CTX: OnceLock<Mutex<Peer>> = OnceLock::new();
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

            let resp;
            {
                // Considering we'll probably need the ctx for other requests/responses I left it outside the match instead of inside GetDiscover
                let mutex = CLIENT_CTX.get().unwrap();
                let mut ctx = mutex.lock().unwrap();
                resp = match req {
                    Request::GetDiscover(peer_info) => {
                        ctx.peers.insert(peer_info.id, peer_info);
                        Response::Discover(ctx.get_info())
                    }
                    Request::Ping => Response::Pong(Utc::now()),
                    Request::Direct(data) => {
                        info!("got dm -> {data}");
                        Response::ACK
                    }
                };
            }
            let _ = write_msg(&mut tx, &resp).await;
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

async fn write_msg<T: Serialize>(
    tx: &mut FramedWrite<SendStream, LengthDelimitedCodec>,
    msg: &T,
) -> anyhow::Result<()> {
    tx.send(postcard::to_allocvec(msg)?.into()).await?;
    Ok(())
}

async fn read_msg<T: DeserializeOwned>(
    rx: &mut FramedRead<RecvStream, LengthDelimitedCodec>,
) -> anyhow::Result<Option<T>> {
    match rx.next().await {
        Some(frame) => Ok(Some(postcard::from_bytes(&frame?)?)),
        None => Ok(None),
    }
}

impl Peer {
    pub async fn send_to(recipient: u8, data: String) -> anyhow::Result<()> {
        let peer;
        {
            let mutex = CLIENT_CTX.get().unwrap();
            let ctx = mutex.lock().unwrap();
            let Some(some_peer) = ctx.peers.get(&recipient) else {
                bail!("Peer not found")
            };
            peer = some_peer.clone();
        }
        let ticket = EndpointTicket::decode_string(&peer.ticket)
            .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;
        let conn = ENDPOINT
            .get()
            .unwrap()
            .connect(ticket.endpoint_addr().clone(), crate::ALPN)
            .await?;
        let (send, recv) = conn.open_bi().await?;
        let (mut tx, mut rx) = (
            FramedWrite::new(send, codec()),
            FramedRead::new(recv, codec()),
        );
        write_msg(&mut tx, &Request::Direct(data)).await?;

        tx.close().await?;

        match read_msg::<Response>(&mut rx).await? {
            Some(Response::ACK) => Ok(()),
            other => bail!("{other:?}"),
        }
    }
    pub async fn discover(ticket: &str) -> anyhow::Result<()> {
        let self_info;
        {
            let mutex = CLIENT_CTX.get().unwrap();
            let ctx = mutex.lock().unwrap();
            self_info = ctx.get_info();
        }

        if self_info.ticket == ticket {
            bail!("Can't connect to yourself");
        }

        let ticket = EndpointTicket::decode_string(ticket)
            .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;
        let conn = ENDPOINT
            .get()
            .unwrap()
            .connect(ticket.endpoint_addr().clone(), crate::ALPN)
            .await?;
        let (send, recv) = conn.open_bi().await?;
        let (mut tx, mut rx) = (
            FramedWrite::new(send, codec()),
            FramedRead::new(recv, codec()),
        );
        write_msg(&mut tx, &Request::GetDiscover(self_info)).await?;

        match read_msg::<Response>(&mut rx).await? {
            Some(Response::Discover(peer_info)) => {
                let mutex = CLIENT_CTX.get().unwrap();
                let mut ctx = mutex.lock().unwrap();
                ctx.peers.insert(peer_info.id, peer_info);
            }
            _ => eprintln!("closed without replying"),
        }
        tx.close().await?;
        Ok(())
    }

    pub async fn ping(ticket: &str) -> anyhow::Result<()> {
        let self_info;
        {
            let mutex = CLIENT_CTX.get().unwrap();
            let ctx = mutex.lock().unwrap();
            self_info = ctx.get_info();
        }

        if self_info.ticket == ticket {
            bail!("Can't connect to yourself");
        }

        let ticket_str = match ticket.parse::<u8>() {
            Ok(peer_id) => {
                let mutex = CLIENT_CTX.get().unwrap();
                let ctx = mutex.lock().unwrap();
                match ctx.peers.get(&peer_id) {
                    Some(peer) => peer.ticket.clone(),
                    None => bail!("Could not find peer"),
                }
            }
            Err(_) => ticket.to_string(),
        };
        let ticket = EndpointTicket::decode_string(&ticket_str)
            .map_err(|e| anyhow!("failed to parse ticket: {}", e))?;
        let conn = ENDPOINT
            .get()
            .unwrap()
            .connect(ticket.endpoint_addr().clone(), crate::ALPN)
            .await?;
        let (send, recv) = conn.open_bi().await?;
        let (mut tx, mut rx) = (
            FramedWrite::new(send, codec()),
            FramedRead::new(recv, codec()),
        );
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
