use std::net::{Ipv4Addr, Ipv6Addr};

pub struct Tracer {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
    pub port: u16,
    pub max_hops: u8,
}

impl Tracer {
    pub fn new(ipv4: Option<Ipv4Addr>, ipv6: Option<Ipv6Addr>, port: u16, max_hops: u8) -> Self {
        Tracer {
            ipv4,
            ipv6,
            port,
            max_hops,
        }
    }
}
