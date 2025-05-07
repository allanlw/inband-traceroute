use std::{fmt, net::IpAddr};

#[derive(Debug, Clone, Copy)]
pub(crate) enum HopType {
    Timeout,
    TcpRst,
    TcpAck,
    IcmpTimeExceeded,
    Origin,
}

impl fmt::Display for HopType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HopType::Timeout => write!(f, "[timeout]"),
            HopType::TcpRst => write!(f, "TCP Reset"),
            HopType::TcpAck => write!(f, "TCP ACK"),
            HopType::IcmpTimeExceeded => write!(f, "ICMP Time Exceeded"),
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
        write!(f, "{}: {}", self.ttl, self.hop_type)?;
        if let Some(addr) = self.addr {
            write!(f, " from {addr}")?;
        };
        if let Some(rtt) = self.rtt {
            write!(f, " (rtt {}ms)", rtt / 1000000)?;
        }
        Ok(())
    }
}
