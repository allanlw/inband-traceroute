use std::{fmt, net::IpAddr};

#[derive(Debug, Clone, Copy)]
pub(crate) enum HopType {
    Timeout,
    TCPRST,
    TCPAck,
    ICMPTimeExceeded,
    Origin,
}

impl fmt::Display for HopType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HopType::Timeout => write!(f, "[timeout]"),
            HopType::TCPRST => write!(f, "TCP Reset"),
            HopType::TCPAck => write!(f, "TCP ACK"),
            HopType::ICMPTimeExceeded => write!(f, "ICMP Time Exceeded"),
            HopType::Origin => write!(f, "[this server]"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Hop {
    pub(crate) ttl: u8,
    pub(crate) hop_type: HopType,
    pub(crate) addr: Option<IpAddr>,
    pub(crate) rtt: Option<u64>,
}

impl fmt::Display for Hop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(addr) = self.addr {
            write!(f, "{}: {} from {}", self.ttl, self.hop_type, addr)
        } else {
            write!(f, "{}: {}", self.ttl, self.hop_type)
        }
    }
}
