use maxminddb::Reader;
use serde::{Deserialize, Serialize};
use std::{fmt, net::IpAddr};

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) enum HopType {
    Timeout,
    TcpRst,
    TcpAck,
    IcmpTimeExceeded,
    Origin,
}

// See https://community.ipinfo.io/t/using-ipinfos-mmdb-database-with-rust/5587
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct IpinfoCountryASN<'a> {
    pub as_domain: Option<&'a str>,
    pub as_name: Option<&'a str>,
    pub asn: Option<&'a str>,
    pub continent: Option<&'a str>,
    pub continent_code: Option<&'a str>,
    pub country: Option<&'a str>,
    pub country_code: Option<&'a str>,
    pub ip: Option<&'a str>,
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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Hop {
    pub(crate) ttl: u8,
    pub(crate) hop_type: HopType,
    pub(crate) addr: Option<IpAddr>,
    pub(crate) rtt: Option<u64>,
    pub(crate) enriched_info: Option<IpinfoCountryASN<'static>>,
}

impl Hop {
    pub(crate) fn new(
        ttl: u8,
        hop_type: HopType,
        addr: Option<IpAddr>,
        rtt: Option<u64>,
        ipdb: &'static Reader<Vec<u8>>,
    ) -> Self {
        Self {
            ttl,
            hop_type,
            addr,
            rtt,
            enriched_info: addr.and_then(|addr| ipdb.lookup::<IpinfoCountryASN>(addr).unwrap()),
        }
    }
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
