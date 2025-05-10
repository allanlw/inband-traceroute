mod ebpf;
mod hop;
mod raw;
mod server;
mod tracer;

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use clap::Parser;
use ebpf::start_event_processor;
use inband_traceroute_common::EbpfConfig;
use log::info;
use tokio::{signal, sync::Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Opt {
    #[arg(short, long, default_value = "eth0")]
    iface: String,

    /// IPv4 address to listen on
    #[arg(long = "ipv4", required_unless_present = "ipv6")]
    ipv4: Option<std::net::Ipv4Addr>,

    /// IPv6 address to listen on
    #[arg(long = "ipv6", required_unless_present = "ipv4")]
    ipv6: Option<std::net::Ipv6Addr>,

    /// Domain for TLS certificate
    #[arg(short, long = "domain", required = true)]
    domain: String,

    /// Contact info for TLS certificate
    #[arg(short, long = "email")]
    emails: Vec<String>,

    /// Cache directory for TLS certificates
    #[arg(short, long)]
    cache_dir: Option<PathBuf>,

    #[arg(short, long, default_value = "443")]
    port: u16,

    /// Use Let's Encrypt production environment
    /// (see https://letsencrypt.org/docs/staging-environment/)
    #[clap(long)]
    prod: bool,
    /// Maximum number of hops
    #[arg(long, default_value = "32")]
    max_hops: u8,

    #[clap(long, default_value = "/opt/ipinfoio/ipinfo_lite.mmdb")]
    ipinfoio_db: PathBuf,

    #[arg(long, default_value="true")]
    v4_v6_subdomains: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("inband_traceroute=error,tower_http=warn"))
                .unwrap(),
        )
        .compact()
        .init();

    info!("Starting inband-traceroute...");

    info!("Using interface: {}", opt.iface);

    info!("Loading IPInfo.io database...");
    let reader = Box::leak(Box::new(
        maxminddb::Reader::open_readfile(&opt.ipinfoio_db)
            .context("Failed to open IPInfo.io database")?,
    ));

    info!("Loading eBPF program...");

    let ebpf_config = EbpfConfig::new(
        opt.port,
        opt.ipv4
            .map(|ipv4| inband_traceroute_common::IPAddr::new_v4(ipv4.octets())),
        opt.ipv6
            .map(|ipv6| inband_traceroute_common::IPAddr::new_v6(ipv6.octets())),
    );

    // Note: program will be detached when dropped
    let (mut ebpf, trace_map) =
        ebpf::setup_ebpf(&opt.iface, &ebpf_config).context("EBPF setup failed")?;

    let trace_map = Arc::new(Mutex::new(trace_map));

    info!("Initializing raw sockets...");

    let tracer_v4 = opt
        .ipv4
        .map(|ipv4| {
            tracer::Tracer::new(
                SocketAddr::new(IpAddr::V4(ipv4), opt.port),
                opt.max_hops,
                trace_map.clone(),
                reader,
            )
        })
        .transpose()
        .context("failed to create IPv4 tracer")?
        .map(Arc::new);

    let tracer_v6 = opt
        .ipv6
        .map(|ipv6| {
            tracer::Tracer::new(
                SocketAddr::new(IpAddr::V6(ipv6), opt.port),
                opt.max_hops,
                trace_map,
                reader,
            )
        })
        .transpose()
        .context("Failed to create IPv6 tracer")?
        .map(Arc::new);

    start_event_processor(&mut ebpf, tracer_v4.clone(), tracer_v6.clone())?;

    let state = Arc::new(server::AppState {
        tracer_v4,
        tracer_v6,
    });

    info!("Setting up server...");

    server::setup_server(&opt, state);

    info!("Access server at https://{}:{}/", opt.domain, opt.port);

    info!("Server started. Press Ctrl+C to stop.");

    let ctrl_c = signal::ctrl_c();
    ctrl_c.await?;

    info!("Shutting down...");

    Ok(())
}
