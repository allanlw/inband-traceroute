use std::net::IpAddr;

pub struct Tracer {
    pub ip: IpAddr,
    pub port: u16,
    pub max_hops: u8,
}

impl Tracer {
    pub fn new(ip: IpAddr, port: u16, max_hops: u8) -> Self {
        Tracer {
            ip,
            port,
            max_hops,
        }
    }
}
