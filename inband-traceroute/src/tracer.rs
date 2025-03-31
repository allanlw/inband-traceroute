use anyhow::Context;
use async_stream::try_stream;
use futures::stream::{self, Stream};
use inband_traceroute_common::{IPAddr, TraceEvent, TraceEventType};
use log::{debug, warn};
use rand::{rngs::OsRng, Rng};
use socket2::Domain;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex, RwLock,
    },
    time::timeout,
};

use crate::{
    ebpf::{self, TraceMap},
    hop::{Hop, HopType},
    raw,
};

type TraceId = u32;

#[derive(Debug)]
pub struct Tracer {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
    pub trace_map: Arc<Mutex<TraceMap>>,

    traces: RwLock<HashMap<TraceId, Weak<TraceHandle>>>,
}

impl Tracer {
    pub fn new(
        listen_addr: SocketAddr,
        max_hops: u8,
        trace_map: Arc<Mutex<TraceMap>>,
    ) -> anyhow::Result<Self> {
        let domain = match listen_addr {
            SocketAddr::V4(_) => Domain::IPV4,
            SocketAddr::V6(_) => Domain::IPV6,
        };

        let socket =
            raw::AsyncWriteOnlyIPRawSocket::new(domain).context("failed to create raw socket")?;

        Ok(Self {
            listen_addr,
            max_hops,
            socket,
            trace_map,
            traces: RwLock::new(HashMap::new()),
        })
    }

    pub async fn process_event(&self, event: TraceEvent) -> anyhow::Result<()> {
        let trace_id = event.trace_id;
        let traces = self.traces.read().await;

        if let Some(trace) = traces.get(&trace_id) {
            if let Some(trace) = trace.upgrade() {
                trace.sender.send(event)?;
            } else {
                warn!("Trace {} is no longer valid", trace_id);
            }
        } else {
            warn!("Trace {} not found", trace_id);
        }
        Ok(())
    }
}

fn std_ipaddr_to_ebpf(addr: std::net::IpAddr) -> IPAddr {
    match addr {
        std::net::IpAddr::V4(ipv4) => IPAddr::new_v4(ipv4.octets()),
        std::net::IpAddr::V6(ipv6) => IPAddr::new_v6(ipv6.octets()),
    }
}

fn std_socket_addr_to_ebpf(addr: SocketAddr) -> inband_traceroute_common::SocketAddr {
    inband_traceroute_common::SocketAddr {
        addr: std_ipaddr_to_ebpf(addr.ip()),
        port: addr.port(),
    }
}

#[derive(Debug)]
pub struct TCPState {
    pub seq: u32,
    pub ack_seq: u32,
}

#[derive(Debug)]
pub struct TraceHandle {
    tracer: Arc<Tracer>, // Must be a strong reference to keep the tracer alive
    trace_id: u32,
    remote: SocketAddr,
    key: inband_traceroute_common::SocketAddr,
    sender: UnboundedSender<TraceEvent>,
    receiver: UnboundedReceiver<TraceEvent>,
}

impl TraceHandle {
    /// Create a new `TraceHandle` and register it
    pub async fn start_trace(tracer: Arc<Tracer>, remote: SocketAddr) -> anyhow::Result<Arc<Self>> {
        let trace_id: u32 = OsRng.gen();

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<TraceEvent>();

        let key = std_socket_addr_to_ebpf(remote);
        let mut res = Arc::new(Self {
            tracer: tracer.clone(),
            trace_id,
            remote,
            key,
            sender,
            receiver,
        });

        {
            let mut traces = tracer.traces.write().await;

            // Just in case of collisions, we will keep generating new trace ids until we find a free one
            while let Some(_) = traces.get(&trace_id) {
                Arc::get_mut(&mut res).unwrap().trace_id = OsRng.gen();
            }

            traces.insert(trace_id, Arc::downgrade(&res));
        }

        {
            let mut trace_map = tracer.trace_map.lock().await;

            debug!("Registering trace id {} for remote {}", trace_id, remote);

            trace_map
                .insert(key, trace_id, 0)
                .context("failed to register trace")?;
        }

        Ok(res)
    }

    async fn wait_for_initial_ack(&self) -> anyhow::Result<(u32, u32)> {
        let mut ack_seq = 0;
        let mut seq = 0;

        loop {
            match self.receiver.recv().await {
                Some(event) => {
                    if event.event_type == TraceEventType::TCP_ACK {
                        ack_seq = event.ack_seq;
                        seq = event.seq;
                        break;
                    } else {
                        warn!("Received unexpected event type: {:?}", event.event_type);
                    }
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "Receiver channel closed before ack was received"
                    ));
                }
            }
        }

        Ok((ack_seq, seq))
    }

    pub async fn hop_stream(&self) -> anyhow::Result<impl Stream<Item = anyhow::Result<Hop>>> {
        let (mut ack_seq, mut seq) =
            timeout(Duration::from_seconds(5), self.wait_for_initial_ack()).await?;

        return Ok(try_stream! {
            yield Hop {
                ttl: 0,
                hop_type: HopType::Origin,
                addr: None,
                rtt: Duration::from_millis(0),
            };

            for ttl in 1..=self.tracer.max_hops {
                // send packet burst

                // wait for response
            }
        });
    }
}

impl Drop for TraceHandle {
    fn drop(&mut self) {
        let trace_map = self.tracer.trace_map.clone();
        let trace_id = self.trace_id;
        let remote = self.remote;
        let key = self.key;

        tokio::spawn(async move {
            let mut trace_map = trace_map.lock().await;
            debug!("Unregistering trace id {} for remote {}", trace_id, remote);

            trace_map.remove(&key).unwrap_or_else(|e| {
                debug!("Failed to unregister trace id {}: {:#?}", trace_id, e);
            });
        });
    }
}
