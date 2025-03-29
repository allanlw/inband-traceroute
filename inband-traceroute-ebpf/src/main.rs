#![no_std]
#![no_main]

use core::mem;

use aya_ebpf::{bindings::xdp_action, bpf_printk, macros::xdp, programs::XdpContext};
use aya_log_ebpf::{info, warn};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{in6_addr, in6_u, IpProto, Ipv4Hdr, Ipv6Hdr},
    tcp::TcpHdr,
};

const PORT: u16 = 443;
const IPV4_LISTEN_ADDR: u32 = 177340418; //  10.146.0.2
const IPV6_LISTEN_ADDR: [u32; 4] = [1638438, 1644253248, 0, 0]; // 2600:1900:4050:162::

#[xdp]
pub fn inband_traceroute(ctx: XdpContext) -> u32 {
    unsafe {
        bpf_printk!(b"received a packet1");
    }

    match try_inband_traceroute(ctx) {
        Ok(_) => xdp_action::XDP_PASS,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

fn try_inband_traceroute(ctx: XdpContext) -> Result<(), ()> {
    let ethhdr: *const EthHdr = ptr_at(&ctx, 0)?;
    let ether_type = unsafe { (*ethhdr).ether_type };

    let mut layer4_protocol: Option<IpProto> = None;
    let mut layer4_offset: Option<usize> = None;

    match ether_type {
        EtherType::Ipv4 => {
            let ipv4hdr: *const Ipv4Hdr = ptr_at(&ctx, EthHdr::LEN)?;
            let dst_addr = u32::from_be(unsafe { (*ipv4hdr).dst_addr });
            if dst_addr != IPV4_LISTEN_ADDR {
                return Ok(());
            }

            layer4_protocol = Some(unsafe { (*ipv4hdr).proto });
            layer4_offset = Some(EthHdr::LEN + Ipv4Hdr::LEN);
        }
        EtherType::Ipv6 => {
            let ipv6hdr: *const Ipv6Hdr = ptr_at(&ctx, EthHdr::LEN)?;
            if unsafe { (*ipv6hdr).dst_addr.in6_u.u6_addr32 } != IPV6_LISTEN_ADDR {
                return Ok(());
            }

            layer4_protocol = Some(unsafe { (*ipv6hdr).next_hdr });
            layer4_offset = Some(EthHdr::LEN + Ipv6Hdr::LEN);
        }
        _ => {
            return Ok(());
        }
    }

    match layer4_protocol {
        Some(IpProto::Tcp) => {
            let tcp_hdr: *const TcpHdr = ptr_at(&ctx, layer4_offset.unwrap())?;
            let dst_port = u16::from_be(unsafe { (*tcp_hdr).dest });
            if dst_port != PORT {
                return Ok(());
            }

            info!(
                &ctx,
                "Received {} TCP packet",
                if ether_type == EtherType::Ipv4 {
                    "IPv4"
                } else {
                    "IPv6"
                }
            );
        }
        _ => {
            return Ok(());
        }
    }

    return Ok(());
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        bpf_printk!(b"PANIC");
    }
    loop {}
}
