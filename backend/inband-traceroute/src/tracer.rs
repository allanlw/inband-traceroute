use core::panic;
use std::{
    collections::HashMap,
    net::{self, IpAddr, Ipv4Addr, SocketAddr, SocketAddrV6},
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::Context;
use async_stream::stream;
use etherparse::{ip_number, Ipv4Header, Ipv6FlowLabel, Ipv6Header, PacketBuilder, TcpHeader};
use futures::stream::{Stream, StreamExt};
use inband_traceroute_common::{IPAddr, TraceEvent, TraceEventType};
use log::{debug, info, warn};
use maxminddb::Reader;
use nix::time::{clock_gettime, ClockId};
use rand::{rngs::OsRng, Rng};
use socket2::{Domain, SockAddr};
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

const STEP_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct Tracer {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
    pub trace_map: Arc<Mutex<TraceMap>>,
    pub ipdb: &'static Reader<Vec<u8>>,

    traces: RwLock<HashMap<TraceId, Weak<TraceHandle>>>,
}

// Equivilent to the bpf_ktime_get_ns function from inside of BPF
fn bpf_ktime_get_ns() -> u64 {
    let ts = clock_gettime(ClockId::CLOCK_MONOTONIC).unwrap();

    (ts.tv_sec() * 1000000000 + ts.tv_nsec())
        .try_into()
        .unwrap()
}

impl Tracer {
    pub fn new(
        listen_addr: SocketAddr,
        max_hops: u8,
        trace_map: Arc<Mutex<TraceMap>>,
        ipdb: &'static Reader<Vec<u8>>,
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
            ipdb,
            traces: RwLock::new(HashMap::new()),
        })
    }

    // We send an outbound TCP Keep Alive Packet
    async fn send_outbound_packet(
        &self,
        addr: SocketAddr,
        ttl: u8,
        seq: u32,
        seq_ack: u32,
    ) -> anyhow::Result<()> {
        let payload: &[u8; 1] = &[0];
        let send_to_addr: SockAddr;

        let ip_header = match (addr.ip(), self.listen_addr.ip()) {
            (IpAddr::V4(remote), IpAddr::V4(local)) => {
                send_to_addr = addr.into();
                etherparse::IpHeaders::Ipv4(
                    {
                        let mut header = Ipv4Header::new(
                            0, // will be overwritteen
                            ttl,
                            ip_number::TCP,
                            local.octets(),
                            remote.octets(),
                        )
                        .unwrap();
                        header.identification = ttl.into();
                        header.dont_fragment = true;
                        header
                    },
                    Default::default(),
                )
            }
            (IpAddr::V6(remote), IpAddr::V6(local)) => {
                // Note port must be set to 0 to avoid EINVAL
                // https://nick-black.com/dankwiki/index.php/Packet_sockets
                send_to_addr = SocketAddrV6::new(remote, 0, 0, 0).into();
                etherparse::IpHeaders::Ipv6(
                    Ipv6Header {
                        hop_limit: ttl,
                        source: local.octets(),
                        destination: remote.octets(),
                        payload_length: 0, // will be overwritten
                        next_header: ip_number::TCP,
                        flow_label: Ipv6FlowLabel::try_new(ttl.into()).unwrap(),
                        ..Default::default()
                    },
                    Default::default(),
                )
            }
            _ => {
                panic!("IP address family mismatch");
            }
        };

        let builder = PacketBuilder::ip(ip_header).tcp_header({
            let mut tcp_header = TcpHeader::new(self.listen_addr.port(), addr.port(), seq, 0xffff);
            tcp_header.psh = true;
            tcp_header.ack = true;
            tcp_header.acknowledgment_number = seq_ack;
            tcp_header
        });

        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));

        builder.write(&mut result, payload).unwrap();

        self.socket
            .send_to(result.as_slice(), &send_to_addr)
            .await?;

        Ok(())
    }

    pub async fn process_event(&self, event: TraceEvent) -> anyhow::Result<()> {
        let trace_id = event.trace_id;
        let traces = self.traces.read().await;

        if let Some(trace) = traces.get(&trace_id) {
            if let Some(trace) = trace.upgrade() {
                trace.sender.send(event)?;
            } else {
                warn!("Trace {trace_id} is no longer valid");
            }
        } else {
            warn!("Trace {trace_id} not found");
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
            while traces.get(&trace_id).is_some() {
                Arc::get_mut(&mut res).unwrap().trace_id = OsRng.gen();
            }

            traces.insert(trace_id, Arc::downgrade(&res));
        }

        {
            let mut trace_map = tracer.trace_map.lock().await;

            debug!("Registering trace id {trace_id} for remote {remote}");

            trace_map
                .insert(key, trace_id, 0)
                .context("failed to register trace")?;
        }

        Ok(res)
    }

    async fn wait_for_initial_ack(&self) -> anyhow::Result<(u32, u32)> {
        let mut receiver = self.receiver.lock().await;

        match receiver.recv().await {
            Some(event) => {
                if event.event_type == TraceEventType::TcpAck {
                    Ok((event.ack_seq, event.seq))
                } else {
                    panic!("Received unexpected event type: {:?}", event.event_type);
                }
            }
            None => Err(anyhow::anyhow!(
                "Receiver channel closed before ack was received"
            )),
        }
    }

    async fn hop_stream_internal<'a>(&'a self) -> anyhow::Result<impl Stream<Item = Hop> + 'a> {
        let (mut ack_seq, mut seq) = timeout(Duration::from_secs(5), self.wait_for_initial_ack())
            .await
            .context("Timed out waiting for initial ACK")?
            .context("Failed to get initial ACK")?;

        let stream = stream! {
                let origin = Hop::new(0, HopType::Origin, Some(self.tracer.listen_addr.ip()), None, self.tracer.ipdb);
                yield origin;

                let mut receiver = self.receiver.lock().await;


              'outer:   for ttl in 1..=self.tracer.max_hops {
                    debug!( "Trace with TTL {ttl}");

                    let sent_seq = ack_seq - 1;
                    // TODO: save timeing information for RTT
                    self.tracer.send_outbound_packet(
                        self.remote,
                        ttl,
                        sent_seq,
                        seq,
                    ).await.expect("Should never fail to send packets");

                    let sent_time = bpf_ktime_get_ns();

                   loop {
                        // TODO: fix timing here to sleep for only remaining timeout if not first run
                        let res = timeout(STEP_TIMEOUT, receiver.recv()).await;

                        debug!("Received event for TTL {ttl}: {res:?}");

                        if res.is_err() {
                            yield Hop::new(
                                ttl,
                                HopType::Timeout,
                                None,
                                None,
                                self.tracer.ipdb
                            );
                            break;
                        }

                        let event = res.unwrap();
                        if event.is_none() {
                            panic!("Receiver channel closed before ack was received");
                        }
                        let event = event.unwrap();
                        match event.event_type {
                            TraceEventType::IcmpTimeExceeded => {
                                yield Hop::new(
                                    ttl,
                                    HopType::IcmpTimeExceeded,
                                    Some(ebpf_to_std_ipaddr(event.addr)),
                                    Some(event.arrival - sent_time),
                                    self.tracer.ipdb
                                );
                            }
                            TraceEventType::TcpAck => {
                                if event.ack_seq - 1 == sent_seq {
                                   yield Hop::new(
                                        ttl,
                                        HopType::TcpAck,
                                        Some(self.remote.ip()),
                                        Some(event.arrival - sent_time),
                                        self.tracer.ipdb
                                    );
                                    break 'outer;
                                } else {
                                    ack_seq = event.ack_seq;
                                    seq = event.seq;
                                }
                            }
                            TraceEventType::TcpRst => {
                                yield  Hop::new(
                                    ttl,
                                    HopType::TcpRst,
                                    Some(self.remote.ip()),
                                    Some(event.arrival - sent_time),
                                    self.tracer.ipdb
                                );
                                break 'outer;
                            }
                        }
                    }
                }
        };

        Ok(stream)
    }

    pub async fn hop_stream<'a>(&'a self) -> anyhow::Result<impl Stream<Item = Hop> + 'a> {
        let mut internal = Box::pin(self.hop_stream_internal().await?);
        let mut trace: Vec<Option<Hop>> = vec![None; self.tracer.max_hops as usize];

        let stream = stream! {
            while let Some(hop) = internal.next().await {
                let ttl = hop.ttl as usize;
                if trace[ttl].is_none() {
                    trace[ttl] = Some(hop.clone());
                    yield hop;
                } else {
                    warn!("Duplicate hop for TTL {ttl}: {hop:?}");
                }
            }
            info!("Trace completed: {trace:?}");
        };

        Ok(stream)
    }
}

impl Drop for TraceHandle {
    fn drop(&mut self) {
        debug!("Dropping trace handle for trace id {}", self.trace_id);
        let trace_id = self.trace_id;
        let remote = self.remote;
        let key = self.key;
        let tracer = self.tracer.clone();

        tokio::spawn(async move {
            {
                let mut trace_map = tracer.trace_map.lock().await;
                debug!("Unregistering trace id {trace_id} for remote {remote}");

                trace_map.remove(&key).unwrap_or_else(|e| {
                    debug!("Failed to unregister trace id {trace_id}: {e:#?}");
                });
            }
            {
                let mut traces = tracer.traces.write().await;
                traces.remove(&trace_id);
            }
        });
    }
}
