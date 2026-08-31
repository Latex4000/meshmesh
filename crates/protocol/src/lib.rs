pub mod codec;
pub mod format;
pub mod state;

use std::println;

use crate::codec::codec;
use crate::format::{Request, Response};
use crate::state::Peer;
use anyhow::{anyhow, bail};
use futures::{SinkExt, StreamExt};
use iroh::endpoint::{RecvStream, SendStream};
use iroh::{endpoint::Connection, protocol::AcceptError};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

use log::info;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
pub const ALPN: &[u8] = b"meshmesh/1";
static ROUTER: tokio::sync::OnceCell<iroh::protocol::Router> = tokio::sync::OnceCell::const_new();
static ENDPOINT: tokio::sync::OnceCell<iroh::Endpoint> = tokio::sync::OnceCell::const_new();
static LOCAL_PEER: tokio::sync::OnceCell<Peer> = tokio::sync::OnceCell::const_new();
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

            let resp = match req {
                Request::GetDiscover => Response::Discover(LOCAL_PEER.get().unwrap().to_owned()),
                Request::Ping => Response::Pong(),
                Request::Direct(data) => {
                    info!("got dm -> {data}");
                    Response::ACK
                }
            };
            write_msg(&mut tx, &resp).await;
            tx.close().await?;
        }

        Ok(())
    }
}

pub async fn init() -> anyhow::Result<Peer> {
    let endpoint = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await?;
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(crate::ALPN, crate::MeshMeshProtocol)
        .spawn();
    endpoint.online().await;
    ROUTER.set(router).ok();

    let ticket = EndpointTicket::new(endpoint.addr());
    ENDPOINT.set(endpoint).ok();
    let peer = Peer::new(ticket.to_string());
    LOCAL_PEER.set(peer.clone()).ok();
    Ok(peer)
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
    pub async fn send_to(recipient: Peer, data: String) -> anyhow::Result<()> {
        let ticket = EndpointTicket::decode_string(&recipient.ticket)
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
            Some(Response::ACK) => return Ok(()),
            other => bail!("{other:?}"),
        };
    }
    pub async fn discover(ticket: &str) -> anyhow::Result<Option<Peer>> {
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
        write_msg(&mut tx, &Request::GetDiscover).await?;

        match read_msg::<Response>(&mut rx).await? {
            Some(Response::Discover(x)) => {
                return Ok(Some(x));
            }
            _ => eprintln!("closed without replying"),
        }
        tx.close().await?;
        Ok(None)
    }
}
