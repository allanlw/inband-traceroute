use anyhow::Context;
use core::{fmt, net::IpAddr};
use hickory_client::{
    client::{Client, ClientHandle},
    proto::{
        rr::{DNSClass, Name, RData, RecordType},
        runtime::TokioRuntimeProvider,
        tcp::TcpClientStream,
    },
};
use std::str::FromStr;
use tokio::sync::Mutex;

pub(crate) struct ReverseDnsProvider {
    client: Mutex<Client>,
}

const UPSTREAM: ([u8; 4], u16) = ([1, 1, 1, 1], 53);

impl ReverseDnsProvider {
    pub async fn new() -> anyhow::Result<Self> {
        let (stream, sender) =
            TcpClientStream::new(UPSTREAM.into(), None, None, TokioRuntimeProvider::new());
        let client = Client::new(stream, sender, None);
        let (client, bg) = client.await?;
        tokio::spawn(bg);
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    pub async fn reverse_lookup(&self, ip: &IpAddr) -> anyhow::Result<String> {
        let mut client = self.client.lock().await;
        // Create a query future
        let response = client
            .query(Self::ip_to_ptr_name(ip), DNSClass::IN, RecordType::PTR)
            .await
            .context("Reverse DNS lookup failed")?;

        if let Some(RData::PTR(name)) = response.answers().get(0).map(|f| f.data()) {
            Ok(name.to_string())
        } else {
            Err(anyhow::anyhow!("Unexpected response type"))
        }
    }

    /// Convert an IP address to the owner-name used in a PTR query
    fn ip_to_ptr_name(ip: &IpAddr) -> Name {
        let s = match ip {
            IpAddr::V4(v4) => {
                // reverse the octets and add the in-addr.arpa suffix
                let oct = v4.octets();
                format!("{}.{}.{}.{}.in-addr.arpa.", oct[3], oct[2], oct[1], oct[0])
            }
            IpAddr::V6(v6) => {
                // expand to nibbles, reverse, add the ip6.arpa suffix
                let mut nibbles = v6
                    .octets()
                    .iter()
                    .flat_map(|b| [b >> 4, b & 0x0f])
                    .map(|n| format!("{:x}", n))
                    .collect::<Vec<_>>();
                nibbles.reverse();
                format!("{}.ip6.arpa.", nibbles.join("."))
            }
        };
        Name::from_str(&s).expect("valid reverse-name")
    }
}

impl fmt::Debug for ReverseDnsProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReverseDnsProvider")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_ip_to_ptr_name_ipv4() {
        let ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let name = ReverseDnsProvider::ip_to_ptr_name(&ip);
        assert_eq!(name.to_string(), "4.3.2.1.in-addr.arpa.");
    }

    #[test]
    fn test_ip_to_ptr_name_ipv6() {
        let ip = IpAddr::V6(Ipv6Addr::new(
            0x2001, 0x0db8, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001,
        ));
        let name = ReverseDnsProvider::ip_to_ptr_name(&ip);
        assert_eq!(
            name.to_string(),
            "1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.8.b.d.0.1.0.0.2.ip6.arpa."
        );
    }
}
