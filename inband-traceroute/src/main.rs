use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use anyhow::Context;
use axum::{routing::get, Router};
use aya::programs::{Xdp, XdpFlags};
use clap::Parser;
use log::{error, info};
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio::signal;
use tokio_stream::StreamExt;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Opt {
    #[arg(short, long, default_value = "eth0")]
    iface: String,

    /// Addresses to listen on
    #[arg(short, long = "address", required = true)]
    addresses: Vec<IpAddr>,

    /// Domains for TLS certificate
    #[arg(short, long = "domain", required = true)]
    domains: Vec<String>,

    /// Contact info for TLS certificate
    #[arg(short, long = "email")]
    emails: Vec<String>,

    /// Cache directory for TLS certificates
    #[arg(short, long)]
    cache_dir: Option<PathBuf>,

    #[arg(short, long, default_value = "443")]
    port: u16,
}

fn setup_ebpf(opt: &Opt) -> anyhow::Result<()> {
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/inband-traceroute"
    )))?;
    aya_log::EbpfLogger::init(&mut ebpf)?;

    let program: &mut Xdp = ebpf.program_mut("inband_traceroute").unwrap().try_into()?;
    program.load()?;
    program.attach(&opt.iface, XdpFlags::SKB_MODE)
        .context("failed to attach the XDP program with default flags - try changing XdpFlags::default() to XdpFlags::SKB_MODE")?;

    Ok(())
}

fn setup_server(opt: &Opt) {
    let mut state = AcmeConfig::new(opt.domains.clone())
        .contact(opt.emails.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(opt.cache_dir.clone().map(DirCache::new))
        .directory(rustls_acme::acme::LETS_ENCRYPT_PRODUCTION_DIRECTORY)
        .state();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => info!("event: {:?}", ok),
                Err(err) => error!("error: {:?}", err),
            }
        }
    });

    for addr in &opt.addresses {
        info!("Listening on {}", addr);
        let service = app.clone().into_make_service();
        let acceptor = acceptor.clone();
        let addr = SocketAddr::new(*addr, opt.port);
        tokio::task::spawn(async move {
            axum_server::bind(addr)
                .acceptor(acceptor)
                .serve(service)
                .await
                .unwrap();
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    env_logger::init();

    setup_ebpf(&opt)?;

    setup_server(&opt);

    println!("Server started. Press Ctrl+C to stop.");

    let ctrl_c = signal::ctrl_c();
    ctrl_c.await?;

    Ok(())
}
