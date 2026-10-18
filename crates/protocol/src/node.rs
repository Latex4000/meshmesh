use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use futures::SinkExt;
use iroh::{
    Endpoint, EndpointAddr, EndpointId, SecretKey,
    endpoint::{Connection, RecvStream, SendStream, presets},
    protocol::Router,
};
use iroh_tickets::endpoint::EndpointTicket;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{
    ALPN, MeshMeshProtocol,
    codec::{Rx, codec, read_msg, write_msg},
    error::Error,
    format::{Request, Response},
    state::{Peer, PeerInfo, State},
};

#[derive(Debug, Clone)]
pub struct Node {
    pub(crate) inner: Arc<Inner>,
    router: Option<Router>,
}
#[derive(Debug)]
pub(crate) struct Inner {
    endpoint: Endpoint,
    state: Mutex<State>,
    pub(crate) conns: Mutex<HashMap<EndpointId, Connection>>,
    events: mpsc::Sender<Event>,
}

#[derive(Debug, Clone)]
pub enum Event {
    DirectMessage { from: EndpointId, text: String },
    PeerDiscovered(EndpointId),
}

impl Node {
    pub fn new(endpoint: Endpoint) -> (Self, MeshMeshProtocol, mpsc::Receiver<Event>) {
        let (events, rx) = mpsc::channel(256);
        let inner = Arc::new(Inner {
            endpoint,
            state: Mutex::new(State::default()),
            conns: Mutex::default(),
            events,
        });

        (
            Node {
                inner: inner.clone(),
                router: None,
            },
            MeshMeshProtocol { inner },
            rx,
        )
    }

    pub fn resolve(node: &Node, prefix: &str) -> Option<EndpointId> {
        let mut hits = node
            .peers()
            .into_iter()
            .filter(|p| p.id.to_string().starts_with(prefix));
        match (hits.next(), hits.next()) {
            (Some(p), None) => Some(p.id),
            _ => None,
        }
    }
    pub fn id(&self) -> EndpointId {
        self.inner.endpoint.id()
    }
    pub fn addr(&self) -> EndpointAddr {
        self.inner.endpoint.addr()
    }
    pub fn ticket(&self) -> String {
        EndpointTicket::new(self.addr()).to_string()
    }
    pub fn peers(&self) -> Vec<PeerInfo> {
        self.inner
            .state
            .lock()
            .unwrap()
            .peers
            .values()
            .cloned()
            .collect::<Vec<PeerInfo>>()
    }
    pub fn forget_peers(&self) {
        self.inner.state.lock().unwrap().peers.clear();
    }
    pub async fn spawn(secret: SecretKey) -> Result<(Self, mpsc::Receiver<Event>), Error> {
        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(secret)
            .bind()
            .await?;

        let (mut node, proto, rx) = Node::new(endpoint.clone());

        node.router = Some(
            Router::builder(endpoint.clone())
                .accept(ALPN, proto)
                .spawn(),
        );
        endpoint.online().await;

        Ok((node, rx))
    }
    pub async fn discover(&self, addr: EndpointAddr) -> Result<(), Error> {
        let me = self.id();
        if addr.id == me {
            return Err(Error::SelfConnectingError);
        }
        let target = addr.id;
        match self
            .inner
            .request(addr.clone(), &Request::GetDiscover(self.addr()))
            .await?
        {
            Response::Peers(list) => {
                let mut st = self.inner.state.lock().unwrap();
                if st
                    .peers
                    .insert(
                        target,
                        PeerInfo {
                            id: target,
                            addr: Some(addr),
                        },
                    )
                    .is_none()
                {
                    self.inner.emit(Event::PeerDiscovered(target));
                }

                for p in list.into_iter().filter(|p| p.id != me) {
                    if let Entry::Vacant(e) = st.peers.entry(p.id) {
                        let id = p.id;
                        e.insert(p);
                        self.inner.emit(Event::PeerDiscovered(id));
                    }
                }
                Ok(())
            }
            other => Err(Error::UnexpectedResponseError(other)),
        }
    }
    pub async fn send_dm(&self, to: EndpointId, text: String) -> Result<(), Error> {
        match self.inner.request(to, &Request::Direct(text)).await? {
            Response::ACK => Ok(()),
            other => Err(Error::UnexpectedResponseError(other)),
        }
    }
    pub async fn ping(&self, to: EndpointId) -> Result<Duration, Error> {
        let start = Instant::now();
        match self.inner.request(to, &Request::Ping).await? {
            Response::Pong(_) => Ok(start.elapsed()),
            other => Err(Error::UnexpectedResponseError(other)),
        }
    }
    pub async fn shutdown(&self) -> Result<(), Error> {
        let ids: Vec<EndpointId> = self
            .inner
            .state
            .lock()
            .unwrap()
            .peers
            .keys()
            .cloned()
            .collect();

        let sends = ids.into_iter().map(|id| {
            let inner = self.inner.clone();
            async move {
                let r = tokio::time::timeout(
                    Duration::from_secs(2),
                    inner.request(id, &Request::Leaving),
                )
                .await;
                if !matches!(r, Ok(Ok(_))) {
                    log::debug!("leaving -> {id}: {r:?}");
                }
            }
        });

        futures::future::join_all(sends).await;
        if let Some(router) = &self.router {
            router.shutdown().await?;
            self.inner.endpoint.close().await;
        }
        Ok(())
    }
}

impl Inner {
    pub(crate) async fn handle_stream(
        &self,
        from: EndpointId,
        send: SendStream,
        recv: RecvStream,
    ) -> Result<(), Error> {
        let mut tx = FramedWrite::new(send, codec());
        let mut rx = FramedRead::new(recv, codec());

        let Some(req) = read_msg::<Request>(&mut rx).await? else {
            return Ok(()); // peer opened a stream and closed it without asking anything
        };

        let response = {
            let mut st = self.state.lock().unwrap();
            match req {
                Request::Ping => Response::Pong(Utc::now()),
                Request::Direct(text) => {
                    self.emit(Event::DirectMessage { from, text });
                    Response::ACK
                }
                Request::GetDiscover(addr) => {
                    let addr = Some(addr).filter(|a| a.id == from);
                    if st.peers.insert(from, PeerInfo { id: from, addr }).is_none() {
                        self.emit(Event::PeerDiscovered(from));
                    }
                    let others = st
                        .peers
                        .values()
                        .filter(|p| p.id != from)
                        .cloned()
                        .collect();
                    Response::Peers(others)
                }
                Request::Leaving => {
                    st.peers.remove(&from);
                    Response::ACK
                }
            }
        };

        write_msg(&mut tx, &response).await?;

        tx.close().await?;
        Ok(())
    }

    fn emit(&self, ev: Event) {
        if let Err(e) = self.events.try_send(ev) {
            log::warn!("event dropped: {e}");
        }
    }

    async fn conn(&self, to: impl Into<EndpointAddr>) -> Result<Connection, Error> {
        let addr = to.into();
        if let Some(c) = self.conns.lock().unwrap().get(&addr.id).cloned()
            && c.close_reason().is_none()
        {
            return Ok(c);
        }
        let c = self.endpoint.connect(addr.clone(), ALPN).await?;
        self.conns.lock().unwrap().insert(addr.id, c.clone());
        Ok(c)
    }

    async fn request(
        &self,
        to: impl Into<EndpointAddr> + Clone,
        req: &Request,
    ) -> Result<Response, Error> {
        let conn = self.conn(to.clone()).await?;
        let (send, recv) = match conn.open_bi().await {
            Ok(s) => s,
            Err(_) => {
                // stale cached conn: evict, retry once
                self.conns.lock().unwrap().remove(&conn.remote_id());
                self.conn(to).await?.open_bi().await?
            }
        };
        let mut tx = FramedWrite::new(send, codec());

        write_msg(&mut tx, req).await?;
        tx.close().await?;

        let mut rx = FramedRead::new(recv, codec());
        match read_msg::<Response>(&mut rx).await? {
            Some(r) => Ok(r),
            None => Err(Error::NoResponseError),
        }
    }
}
