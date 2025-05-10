#![no_std]

// This file contains types that are passed through perf maps

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct TraceEvent {
    pub arrival: u64,
    pub trace_id: u32,
    pub ack_seq: u32,
    pub seq: u32,
    pub event_type: TraceEventType,
    pub ip_version: IPVersion,
    pub ttl: u8,
    pub addr: IPAddr,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TraceEventType {
    TcpAck,
    TcpRst,
    IcmpTimeExceeded,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum IPVersion {
    #[default]
    EMPTY = 0,
    IPV4 = 4,
    IPV6 = 6,
}

/// Expect this to be wire format
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct IPAddr {
    pub ip_version: IPVersion,
    pub addr: [u8; 16],
}

impl IPAddr {
    #[inline(always)]
    pub fn new_v4(short_addr: [u8; 4]) -> Self {
        // Convert the 4-byte address to a 16-byte address
        let mut addr = [0; 16];
        addr[0..4].copy_from_slice(&short_addr);
        Self {
            addr,
            ip_version: IPVersion::IPV4,
        }
    }

    #[inline(always)]
    pub fn new_v6(addr: [u8; 16]) -> Self {
        Self {
            addr,
            ip_version: IPVersion::IPV6,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SocketAddr {
    pub port: u16,
    pub addr: IPAddr,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct EbpfConfig {
    pub port: u16,
    ipv4: IPAddr,
    ipv6: IPAddr,
}

impl EbpfConfig {
    pub fn new(port: u16, ipv4: Option<IPAddr>, ipv6: Option<IPAddr>) -> Self {
        Self {
            port,
            ipv4: ipv4.unwrap_or_default(),
            ipv6: ipv6.unwrap_or_default(),
        }
    }

    #[inline(always)]
    pub fn get_ipv4(&self) -> Option<u32> {
        if self.ipv4.ip_version == IPVersion::IPV4 {
            self.ipv4.addr[0..4].try_into().map(u32::from_le_bytes).ok()
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_ipv6(&self) -> Option<[u8; 16]> {
        if self.ipv6.ip_version == IPVersion::IPV6 {
            Some(self.ipv6.addr)
        } else {
            None
        }
    }
}

#[cfg(feature = "user")]
mod userspace {
    unsafe impl aya::Pod for crate::IPAddr {}

    unsafe impl aya::Pod for crate::SocketAddr {}

    unsafe impl aya::Pod for crate::EbpfConfig {}
}

#[cfg(test)]
mod tests {
    use core::mem;

    use super::*;

    #[test]
    fn test_trace_event_size() {
        assert_eq!(mem::size_of::<TraceEvent>(), 40);
    }

    #[test]
    fn test_ipaddr_size() {
        assert_eq!(mem::size_of::<IPAddr>(), 17);
    }

    #[test]
    fn test_socketaddr_size() {
        assert_eq!(mem::size_of::<SocketAddr>(), 19);
    }

    #[test]
    fn test_ebpf_config_size() {
        assert_eq!(mem::size_of::<EbpfConfig>(), 36);
    }
}
