#![no_std]
#![no_main]

use core::mem;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerfEventArray},
    programs::XdpContext,
};
use aya_log_ebpf::info;
use inband_traceroute_common::{IPAddr, IPVersion, SocketAddr, TraceEvent};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr, Ipv6Hdr},
    tcp::TcpHdr,
};

const PORT: u16 = 443;
const IPV4_LISTEN_ADDR: u32 = u32::from_be((10 << 24) + (146 << 16) + (0 << 8) + (2)); //  10.146.0.2
const IPV6_LISTEN_ADDR: [u32; 4] = [1638438, 1644253248, 0, 0]; // 2600:1900:4050:162::

const MAX_TRACES: u32 = 1024;

#[map]
static EVENTS: PerfEventArray<TraceEvent> = PerfEventArray::new(0);

#[map]
static TRACES: HashMap<SocketAddr, u32> = HashMap::with_max_entries(MAX_TRACES, 0);

#[xdp]
pub fn inband_traceroute(ctx: XdpContext) -> u32 {
    match try_inband_traceroute(ctx) {
        Ok(_) => xdp_action::XDP_PASS,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

// Basically we ignore all packets that are not destined for our server (protocol, address, port)
// Then, ignore all packets that are not associated with an active trace
fn try_inband_traceroute(ctx: XdpContext) -> Result<(), ()> {
    let ethhdr: &EthHdr = ptr_at(&ctx, 0)?;
    let ether_type = ethhdr.ether_type;

    let mut src_addr: SocketAddr = SocketAddr::default();


    match ether_type {
        EtherType::Ipv4 => {
            let ipv4hdr: &Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
            let dst_addr = ipv4hdr.dst_addr;
            if dst_addr != IPV4_LISTEN_ADDR {
                return Ok(());
            }

            ip_version = IPVersion::IPV4;
            layer4_protocol = Some(ipv4hdr.proto);
            layer4_offset = Some(EthHdr::LEN + Ipv4Hdr::LEN);

            src_addr.addr = IPAddr::new_v4(ipv4hdr.src_addr.to_le_bytes());
        }
        EtherType::Ipv6 => {
            let ipv6hdr: &Ipv6Hdr = ptr_at(&ctx, EthHdr::LEN)?;
            if unsafe { ipv6hdr.dst_addr.in6_u.u6_addr32 } != IPV6_LISTEN_ADDR {
                return Ok(());
            }

            ip_version = IPVersion::IPV6;
            layer4_protocol = Some(ipv6hdr.next_hdr);
            layer4_offset = Some(EthHdr::LEN + Ipv6Hdr::LEN);

            src_addr.addr = IPAddr::new_v6(unsafe { ipv6hdr.src_addr.in6_u.u6_addr8 });
        }
        _ => {
            return Ok(());
        }
    }

    match layer4_protocol {
        Some(IpProto::Tcp) => {
            let tcp_hdr: &TcpHdr = ptr_at(&ctx, layer4_offset.unwrap())?;
            let dst_port = u16::from_be(tcp_hdr.dest);

            if dst_port != PORT {
                return Ok(());
            }

            // Ignore packets that are not TCP SYN or RST now to avoid map lookups
            if tcp_hdr.ack() == 0 && tcp_hdr.rst() == 0 {
                return Ok(());
            }

            src_addr.port = u16::from_be(tcp_hdr.source);
            let trace_id = unsafe { TRACES.get(&src_addr) };
            match trace_id {
                None => {
                    return Ok(());
                }
                Some(trace_id) => {
                    // Found a trace, send event
                    let event = TraceEvent {
                        trace_id: *trace_id,
                        event_type: if tcp_hdr.ack() != 0 {
                            inband_traceroute_common::TraceEventType::TcpAck
                        } else {
                            inband_traceroute_common::TraceEventType::TcpRst
                        },
                        ack_seq: u32::from_be(tcp_hdr.ack_seq),
                        seq: u32::from_be(tcp_hdr.seq),
                        ip_version: ip_version,
                    };

                    info!(&ctx, "Sending event: {}", event.trace_id);

                    EVENTS.output(&ctx, &event, 0)
                }
            }

            return Ok(());
        }
        _ => {
            return Ok(());
        }
    }
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<&T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    // Safety: Verified to be in bounds by the above check
    Ok(unsafe { &*((start + offset) as *const T) })
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
