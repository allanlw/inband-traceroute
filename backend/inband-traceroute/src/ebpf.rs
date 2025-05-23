use core::panic;
use std::sync::Arc;

use anyhow::Context;
use aya::{
    maps::{Array, AsyncPerfEventArray, HashMap, MapData},
    programs::{Xdp, XdpFlags},
    util::online_cpus,
};
use bytes::BytesMut;
use inband_traceroute_common::{EbpfConfig, TraceEvent};
use log::{debug, warn};
use tokio::task;

use crate::tracer::Tracer;

pub(crate) type EventMap = AsyncPerfEventArray<MapData>;
pub(crate) type TraceMap = HashMap<MapData, inband_traceroute_common::SocketAddr, u32>;

pub(crate) fn setup_ebpf(
    iface: &str,
    config: &EbpfConfig,
) -> anyhow::Result<(aya::Ebpf, TraceMap)> {
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/inband-traceroute"
    )))
    .context("Failed to load program")?;

    aya_log::EbpfLogger::init(&mut ebpf)?;

    let program: &mut Xdp = ebpf.program_mut("inband_traceroute").unwrap().try_into()?;

    program.load().context("Failed to load program")?;
    program
        .attach(iface, XdpFlags::SKB_MODE)
        .context("failed to attach the XDP program - wrong mode?")?;

    {
        let mut config_map: Array<MapData, EbpfConfig> =
            Array::try_from(ebpf.take_map("CONFIG").expect("failed to find CONFIG map"))?;
        config_map
            .set(0, config, 0)
            .context("failed to set CONFIG map")?;
    }

    let trace_map: TraceMap =
        HashMap::try_from(ebpf.take_map("TRACES").expect("failed to find TRACES map"))?;

    Ok((ebpf, trace_map))
}

pub(crate) fn start_event_processor(
    ebpf: &mut aya::Ebpf,
    tracer_v4: Option<Arc<Tracer>>,
    tracer_v6: Option<Arc<Tracer>>,
) -> anyhow::Result<()> {
    let mut event_map: EventMap =
        AsyncPerfEventArray::try_from(ebpf.take_map("EVENTS").expect("failed to find EVENTS map"))
            .expect("failed to create EVENTS map");

    for cpu_id in online_cpus().map_err(|(_, error)| error)? {
        let mut buf = event_map.open(cpu_id, None)?;

        let tracer_v4 = tracer_v4.clone();
        let tracer_v6 = tracer_v6.clone();

        task::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(1024))
                .collect::<Vec<_>>();

            loop {
                let events = buf
                    .read_events(&mut buffers)
                    .await
                    .expect("Reading from perf buffer should never fail");

                for buf in buffers.iter_mut().take(events.read) {
                    let ptr = buf.as_ptr() as *const TraceEvent;
                    let data = unsafe { ptr.read_unaligned() };
                    debug!("received event from ebpf: {data:?}");

                    let tracer = match data.ip_version {
                        inband_traceroute_common::IPVersion::IPV4 => &tracer_v4,
                        inband_traceroute_common::IPVersion::IPV6 => &tracer_v6,
                        inband_traceroute_common::IPVersion::EMPTY => panic!("invalid IP version"),
                    };

                    let res = tracer
                        .as_ref()
                        .expect("tracer should be present")
                        .process_event(data)
                        .await;

                    if let Err(err) = res {
                        warn!("Error processing event: {data:?} {err:?}");
                    }
                }
            }
        });
    }

    Ok(())
}
