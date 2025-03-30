use std::net::{Ipv4Addr, Ipv6Addr};

pub struct Tracer<IP> {
    pub ip: IP,
    pub port: u16,
    pub max_hops: u8,
}

impl<IP> Tracer<IP> {
    pub fn new(ip: IP, port: u16, max_hops: u8) -> Self {
        Tracer {
            ip,
            port,
            max_hops,
        }
    }
}
