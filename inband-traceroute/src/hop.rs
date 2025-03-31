use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug)]
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

#[derive(Debug)]
pub(crate) struct Hop {
    pub(crate) ttl: u8,
    pub(crate) hop_type: HopType,
    pub(crate) addr: Option<IpAddr>,
    pub(crate) rtt: Duration,
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
