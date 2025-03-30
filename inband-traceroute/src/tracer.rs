mod raw;

use libc::{IPPROTO_RAW, SOCK_RAW};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;

pub struct Tracer<IP: Into<IpAddr> + Copy> {
    pub listen_addr: SocketAddr,
    pub max_hops: u8,
    pub socket: raw::AsyncWriteOnlyIPRawSocket,
}

impl<IP: Into<IpAddr> + std::marker::Copy> Tracer<IP> {
    pub fn new(listen_addr: SocketAddr, max_hops: u8) -> anyhow::Result<Self> {
        let socket = raw::AsyncWriteOnlyIPRawSocket::new(domain)?;

        Ok(Self {
            listen_addr,
            max_hops,
            socket,
        })
    }
}
