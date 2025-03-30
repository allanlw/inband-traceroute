use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tokio::net::UdpSocket;

pub struct Tracer<IP: Into<IpAddr>> {
    pub ip: IP,
    pub port: u16,
    pub max_hops: u8,
    pub socket: UdpSocket,
}

impl<IP: Into<IpAddr>> Tracer<IP> {
    pub async fn new(ip: IP, port: u16, max_hops: u8) -> tokio::io::Result<Self> {
        let domain = match ip.into() {
            IpAddr::V4(_) => Domain::IPV4,
            IpAddr::V6(_) => Domain::IPV6,
        };
        let socket = Socket::new(domain, Type::RAW, Some(Protocol::ICMPV4))?;
        socket.set_nonblocking(true)?;
        let std_socket = std::net::UdpSocket::from(socket);
        let socket = UdpSocket::from_std(std_socket)?;

        Ok(Tracer {
            ip,
            port,
            max_hops,
            socket,
        })
    }
}

impl<IP> Tracer<IP> {
    pub fn new(ip: IP, port: u16, max_hops: u8) -> Self {
        Tracer { ip, port, max_hops }
    }
}
