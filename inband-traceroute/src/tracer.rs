use anyhow::Context;
use inband_traceroute_common::IPAddr;
use log::debug;
use rand::{rngs::OsRng, Rng};
use socket2::Domain;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    ebpf::{self, TraceMap},
    raw,
};

#[derive(Debug)]
pub struct Tracer {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
    pub trace_map: Arc<Mutex<TraceMap>>,
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
        })
    }
}

#[derive(Debug)]
pub struct TraceHandle {
    tracer: Arc<Tracer>,
    trace_id: u32,
    remote: SocketAddr,
    key: inband_traceroute_common::SocketAddr,
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

impl TraceHandle {
    /// Create a new `TraceHandle` and register it
    pub async fn start_trace(tracer: Arc<Tracer>, remote: SocketAddr) -> anyhow::Result<Self> {
        let trace_id: u32 = OsRng.gen();

        let key = std_socket_addr_to_ebpf(remote);

        {
            let mut trace_map = tracer.trace_map.lock().await;

            debug!("Registering trace id {} for remote {}", trace_id, remote);

            trace_map
                .insert(key, trace_id, 0)
                .context("failed to register trace")?;
        }

        Ok(Self {
            tracer,
            trace_id,
            remote,
            key,
        })
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
