#![no_std]

// This file contains types that are passed through perf maps

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct TraceEvent {
    pub trace_id: u32,
    pub ack_seq: u32,
    pub seq: u32,
    pub event_type: TraceEventType,
    pub ip_version: IPVersion,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TraceEventType {
    TcpAck,
    TcpRst,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Default)]
pub enum IPVersion {
    #[default]
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
            addr: addr,
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
#[cfg(feature = "user")]
unsafe impl aya::Pod for IPAddr {}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SocketAddr {
    pub port: u16,
    pub addr: IPAddr,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for SocketAddr {}

#[cfg(test)]
mod tests {
    use core::mem;

    use super::*;

    #[test]
    fn test_trace_event_size() {
        assert_eq!(mem::size_of::<TraceEvent>(), 14);
    }

    #[test]
    fn test_ipaddr_size() {
        assert_eq!(mem::size_of::<IPAddr>(), 17);
    }

    #[test]
    fn test_socketaddr_size() {
        assert_eq!(mem::size_of::<SocketAddr>(), 19);
    }
}
