use anyhow::{Context, anyhow};
use futures::{SinkExt, StreamExt};
use iroh::endpoint::{RecvStream, SendStream};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::ENDPOINT;

pub type Tx = FramedWrite<SendStream, LengthDelimitedCodec>;
pub type Rx = FramedRead<RecvStream, LengthDelimitedCodec>;

pub fn codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .big_endian()
        .max_frame_length(1 << 20)
        .new_codec()
}

pub async fn open_stream(ticket: &str) -> anyhow::Result<(Tx, Rx)> {
    let ticket = EndpointTicket::decode_string(ticket)
        .map_err(|e| anyhow!("failed to parse ticket: {e}"))?;
    let conn = ENDPOINT
        .get()
        .context("endpoint has not been initialised")?
        .connect(ticket.endpoint_addr().clone(), crate::ALPN)
        .await?;
    let (send, recv) = conn.open_bi().await?;
    Ok((
        FramedWrite::new(send, codec()),
        FramedRead::new(recv, codec()),
    ))
}

pub async fn write_msg<T: Serialize>(tx: &mut Tx, msg: &T) -> anyhow::Result<()> {
    tx.send(postcard::to_allocvec(msg)?.into()).await?;
    Ok(())
}

pub async fn read_msg<T: DeserializeOwned>(rx: &mut Rx) -> anyhow::Result<Option<T>> {
    match rx.next().await {
        Some(frame) => Ok(Some(postcard::from_bytes(&frame?)?)),
        None => Ok(None),
    }
}
