use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_stream::stream;
use axum::{
    body::{Body, Bytes},
    response::Response,
    routing::get,
    Router,
};
use aya::programs::{Xdp, XdpFlags};
use clap::Parser;
use log::{error, info};
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio::{signal, time::sleep};
use tokio_stream::StreamExt;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;

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

    /// Use Let's Encrypt production environment
    /// (see https://letsencrypt.org/docs/staging-environment/)
    #[clap(long)]
    prod: bool,
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

async fn index_handler() -> Response {
    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=UTF-8")
        .body(Body::from_stream(stream! {
            yield Ok::<Bytes, anyhow::Error>(Bytes::from_static(b"Hello, world!\n"));
            sleep(Duration::from_secs(3)).await;
            yield  Ok::<Bytes, anyhow::Error>(Bytes::from_static(b"This is a simple HTTP server.\n"));
            sleep(Duration::from_secs(3)).await;
            yield  Ok::<Bytes, anyhow::Error>(Bytes::from_static(b"It supports HTTP/2 and HTTP/1.1.\n"));
        }))
        .unwrap()
}

fn setup_server(opt: &Opt) {
    let mut state = AcmeConfig::new(opt.domains.clone())
        .contact(opt.emails.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(opt.cache_dir.clone().map(DirCache::new))
        .directory_lets_encrypt(opt.prod)
        .state();

    let app = Router::new().route("/", get(index_handler)).layer(
        TraceLayer::new_for_http()
            .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
            .on_response(
                trace::DefaultOnResponse::new()
                    .level(Level::INFO)
                    .include_headers(true),
            )
            .on_request(trace::DefaultOnRequest::new().level(Level::INFO)),
    );

    let mut rustls_config = state.default_rustls_config();
    Arc::get_mut(&mut rustls_config).unwrap().alpn_protocols =
        vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let acceptor = state.axum_acceptor(rustls_config);

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

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("inband_traceroute=error,tower_http=warn"))
                .unwrap(),
        )
        .compact()
        .init();

    setup_ebpf(&opt)?;

    setup_server(&opt);

    println!("Server started. Press Ctrl+C to stop.");

    let ctrl_c = signal::ctrl_c();
    ctrl_c.await?;

    Ok(())
}
