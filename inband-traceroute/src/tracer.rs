use core::panic;
use std::{
    collections::HashMap,
    net::{self, IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::Context;
use async_stream::{stream, try_stream};
use etherparse::{ip_number, Ipv4Header, Ipv6Header, PacketBuilder};
use futures::{stream::Stream, SinkExt};
use inband_traceroute_common::{IPAddr, TraceEvent, TraceEventType};
use log::{debug, info, warn};
use rand::{rngs::OsRng, Rng};
use socket2::Domain;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex, RwLock,
    },
    time::timeout,
};

use crate::{
    ebpf::TraceMap,
    hop::{Hop, HopType},
    raw,
};

type TraceId = u32;

const STEP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Tracer {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
    pub trace_map: Arc<Mutex<TraceMap>>,

    domain: String,
    traces: RwLock<HashMap<TraceId, Weak<TraceHandle>>>,
}

impl Tracer {
    pub fn new(
        listen_addr: SocketAddr,
        max_hops: u8,
        trace_map: Arc<Mutex<TraceMap>>,
        hostname: String,
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
            domain: hostname,
            traces: RwLock::new(HashMap::new()),
        })
    }

    pub async fn send_outbound_packet_burst(
        &self,
        addr: SocketAddr,
        ttl: u8,
        seq: u32,
        ack: u32,
    ) -> anyhow::Result<()> {
        for x in 0..=1 {
            for y in 0..=1 {
                self.send_outbound_packet(addr, ttl, seq - x, ack - y)
                    .await?;
            }
        }
        Ok(())
    }

    async fn send_outbound_packet(
        &self,
        addr: SocketAddr,
        ttl: u8,
        seq: u32,
        seq_ack: u32,
    ) -> anyhow::Result<()> {
        let payload = self.domain.as_bytes();
        let builder = PacketBuilder::ip(match (addr.ip(), self.listen_addr.ip()) {
            (IpAddr::V4(remote), IpAddr::V4(local)) => etherparse::IpHeaders::Ipv4(
                {
                    let mut header = Ipv4Header::new(
                        payload.len().try_into().unwrap(),
                        ttl,
                        ip_number::TCP,
                        local.octets(),
                        remote.octets(),
                    )
                    .unwrap();
                    header.identification = ttl.try_into().unwrap();
                    header.dont_fragment = true;
                    header
                },
                Default::default(),
            ),
            (IpAddr::V6(remote), IpAddr::V6(local)) => etherparse::IpHeaders::Ipv6(
                Ipv6Header {
                    hop_limit: ttl,
                    source: local.octets(),
                    destination: remote.octets(),
                    payload_length: payload.len().try_into().unwrap(),
                    next_header: ip_number::TCP,
                    ..Default::default()
                },
                Default::default(),
            ),
            _ => {
                panic!("IP address family mismatch");
            }
        })
        .tcp(self.listen_addr.port(), addr.port(), seq, 0xffff)
        .ack(seq_ack)
        .psh();

        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));

        builder.write(&mut result, &payload).unwrap();

        self.socket.send_to(result.as_slice(), &addr.into()).await?;

        Ok(())
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

fn std_ipaddr_to_ebpf(addr: IpAddr) -> IPAddr {
    match addr {
        IpAddr::V4(ipv4) => IPAddr::new_v4(ipv4.octets()),
        IpAddr::V6(ipv6) => IPAddr::new_v6(ipv6.octets()),
    }
}

fn std_socket_addr_to_ebpf(addr: SocketAddr) -> inband_traceroute_common::SocketAddr {
    inband_traceroute_common::SocketAddr {
        addr: std_ipaddr_to_ebpf(addr.ip()),
        port: addr.port(),
    }
}

fn ebpf_to_std_ipaddr(addr: IPAddr) -> IpAddr {
    match addr.ip_version {
        inband_traceroute_common::IPVersion::IPV4 => IpAddr::V4(Ipv4Addr::new(
            addr.addr[0],
            addr.addr[1],
            addr.addr[2],
            addr.addr[3],
        )),
        inband_traceroute_common::IPVersion::IPV6 => IpAddr::V6(net::Ipv6Addr::from(addr.addr)),
        inband_traceroute_common::IPVersion::EMPTY => panic!("Empty IP address"),
    }
}

#[derive(Debug)]
pub struct TraceHandle {
    tracer: Arc<Tracer>, // Must be a strong reference to keep the tracer alive
    trace_id: u32,
    remote: SocketAddr,
    key: inband_traceroute_common::SocketAddr,
    sender: UnboundedSender<TraceEvent>,
    receiver: Mutex<UnboundedReceiver<TraceEvent>>,
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
            receiver: Mutex::new(receiver),
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

        let mut receiver = self.receiver.lock().await;

        loop {
            match receiver.recv().await {
                Some(event) => {
                    if event.event_type == TraceEventType::TcpAck {
                        ack_seq = event.ack_seq;
                        seq = event.seq;
                        break;
                    } else {
                        panic!("Received unexpected event type: {:?}", event.event_type);
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

    pub async fn hop_stream<'a>(&'a self) -> anyhow::Result<impl Stream<Item = Hop> + 'a> {
        let (mut ack_seq, mut seq) = timeout(Duration::from_secs(5), self.wait_for_initial_ack())
            .await
            .context("Timed out waiting for initial ACK")?
            .context("Failed to get initial ACK")?;

        let mut trace: Vec<Option<Hop>> = vec![None; self.tracer.max_hops as usize];

        let stream = stream! {
                let origin = Hop {
                    ttl: 0,
                    hop_type: HopType::Origin,
                    addr: Some(self.tracer.listen_addr.ip()),
                    rtt: Duration::from_millis(0),
                };

                trace[0] = Some(origin);
                yield origin;

                let mut receiver = self.receiver.lock().await;

                for ttl in 1..=self.tracer.max_hops {
                    info!( "Trace with TTL {}", ttl);
                    self.tracer.send_outbound_packet_burst(
                        self.remote,
                        ttl,
                        ack_seq,
                        seq,
                    ).await.unwrap();

                    loop {
                        let res = timeout(STEP_TIMEOUT, receiver.recv()).await;

                        info!("Received event for TTL {}: {:?}", ttl, res);

                        if let Err(_) = res {
                            let x = Hop {
                                    ttl,
                                    hop_type: HopType::Timeout,
                                    addr: None,
                                    rtt: Duration::from_millis(0),
                            };
                            trace[ttl as usize] = Some(x);
                            yield x;
                            break;
                        }

                        let event = res.unwrap();
                        if event.is_none() {
                            panic!("Receiver channel closed before ack was received");
                        }
                        let event = event.unwrap();
                        match event.event_type {
                            TraceEventType::IcmpTimeExceeded => {
                                let x = Hop {
                                    ttl: event.ttl,
                                    hop_type: HopType::ICMPTimeExceeded,
                                    addr: Some(ebpf_to_std_ipaddr(event.addr)),
                                    rtt: Duration::from_millis(0),
                                };
                                if trace[x.ttl as usize].is_none() {
                                    trace[x.ttl as usize] = Some(x);
                                    yield x;
                                    break;
                                } else {
                                    warn!("Received duplicate ICMP Time Exceeded event for TTL {}", ttl);
                                }
                            }
                            TraceEventType::TcpAck => {
                                ack_seq = event.ack_seq;
                                seq = event.seq;
                            }
                            TraceEventType::TcpRst => {
                                let x = Hop {
                                    ttl,
                                    hop_type: HopType::TCPRST,
                                    addr: Some(self.remote.ip()),
                                    rtt: Duration::from_millis(0),
                                };
                                if trace[ttl as usize].is_none() {
                                    trace[ttl as usize] = Some(x);
                                    yield x;
                                    break;
                                } else {
                                    warn!("Received duplicate TCP RST event for TTL {}", ttl);
                                }
                            }
                            _ => {
                                panic!("Received unexpected event type: {:?}", event.event_type);
                            }
                        }
                    }
                }
                warn!("Trace completed: {:?}", trace);
        };

        return Ok(stream);
    }
}

impl Drop for TraceHandle {
    fn drop(&mut self) {
        info!("Dropping trace handle for trace id {}", self.trace_id);
        let trace_id = self.trace_id;
        let remote = self.remote;
        let key = self.key;
        let tracer = self.tracer.clone();

        tokio::spawn(async move {
            {
                let mut trace_map = tracer.trace_map.lock().await;
                debug!("Unregistering trace id {} for remote {}", trace_id, remote);

                trace_map.remove(&key).unwrap_or_else(|e| {
                    debug!("Failed to unregister trace id {}: {:#?}", trace_id, e);
                });
            }
            {
                let mut traces = tracer.traces.write().await;
                traces.remove(&trace_id);
            }
        });
    }
}
