mod raw;

use anyhow::Context;
use log::info;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, SocketAddr};

#[derive(Debug)]
pub struct Tracer {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
}

impl Tracer {
    pub fn new(listen_addr: SocketAddr, max_hops: u8) -> anyhow::Result<Self> {
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
        })
    }
}
