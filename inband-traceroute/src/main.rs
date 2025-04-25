mod ebpf;
mod hop;
mod raw;
mod tracer;

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_stream::{stream, try_stream};
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, State},
    response::Response,
    routing::get,
    Router,
};
use clap::Parser;
use ebpf::start_event_processor;
use futures::Stream;
use hyper::body::Frame;
use log::{error, info, warn};
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio::{signal, sync::Mutex, time::sleep};
use tokio_stream::StreamExt;
use tower_http::trace::{self, TraceLayer};
use tracer::TraceHandle;
use tracing::Level;
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
}

#[derive(Debug)]
struct AppState {
    tracer_v4: Option<Arc<tracer::Tracer>>,
    tracer_v6: Option<Arc<tracer::Tracer>>,
}

// TODO: Fix panics
async fn index_handler<'a>(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> Response {
    let tracer = match remote {
        SocketAddr::V4(_) => state.tracer_v4.clone(),
        SocketAddr::V6(_) => state.tracer_v6.clone(),
    }
    .expect("If we got a connection in this protocol, the program should have a tracer for it");

    info!("Remote: {:?}", remote);

    let trace_handle = TraceHandle::start_trace(tracer, remote).await.unwrap();

    let stream = try_stream! {
        let mut hop_stream = Box::pin(trace_handle.hop_stream().await.unwrap());
        warn!("Trace started");
        while let Some(hop) = hop_stream.next().await {
            warn!("Got hop: {:?}", hop);
            let hop = format!("{:?}\n", hop);
            yield hop.into();
            yield Bytes::from_static(b"<br>\n");
        }

        warn!("Trace finished");

        yield format!("{:?}", trace_handle).into();
        sleep(Duration::from_secs(3)).await;
        yield Bytes::from_static(b"This is a simple HTTP server.\n");
        sleep(Duration::from_secs(3)).await;
        yield Bytes::from_static(b"It supports HTTP/2 and HTTP/1.1.\n");
    };

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=UTF-8")
        .body(Body::from_stream(stream.map(
            |b: anyhow::Result<Bytes>| -> anyhow::Result<Bytes> { b },
        )))
        .unwrap()
}

fn setup_server(opt: &Opt, state: Arc<AppState>) {
    let mut acme_state = AcmeConfig::new(vec![opt.domain.clone()])
        .contact(opt.emails.iter().map(|e| format!("mailto:{}", e)))
        .cache_option(opt.cache_dir.clone().map(DirCache::new))
        .directory_lets_encrypt(opt.prod)
        .state();

    let app = Router::new()
        .route("/", get(index_handler))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(
                    trace::DefaultOnResponse::new()
                        .level(Level::INFO)
                        .include_headers(true),
                )
                .on_request(trace::DefaultOnRequest::new().level(Level::INFO)),
        );

    let mut rustls_config = acme_state.default_rustls_config();
    Arc::get_mut(&mut rustls_config).unwrap().alpn_protocols =
        vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let acceptor = acme_state.axum_acceptor(rustls_config);

    tokio::spawn(async move {
        loop {
            match acme_state.next().await.unwrap() {
                Ok(ok) => info!("event: {:?}", ok),
                Err(err) => error!("error: {:?}", err),
            }
        }
    });

    let addresses: Vec<SocketAddr> = [opt.ipv4.map(IpAddr::V4), opt.ipv6.map(IpAddr::V6)]
        .into_iter()
        .filter_map(|ip| ip.map(|ip| SocketAddr::new(ip, opt.port)))
        .collect();

    for addr in addresses {
        info!("Listening on {}", addr);
        let service = app
            .clone()
            .into_make_service_with_connect_info::<SocketAddr>();
        let acceptor = acceptor.clone();
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

    info!("Starting inband-traceroute...");

    info!("Using interface: {}", opt.iface);

    info!("Loading eBPF program...");

    // Note: program will be detached when dropped
    let (mut ebpf, trace_map) = ebpf::setup_ebpf(&opt.iface).context("EBPF setup failed")?;

    let trace_map = Arc::new(Mutex::new(trace_map));

    info!("Initializing raw sockets...");

    let tracer_v4 = opt
        .ipv4
        .map(|ipv4| {
            tracer::Tracer::new(
                SocketAddr::new(IpAddr::V4(ipv4), opt.port),
                opt.max_hops,
                trace_map.clone(),
                opt.domain.clone(),
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
                opt.domain.clone(),
            )
        })
        .transpose()
        .context("Failed to create IPv6 tracer")?
        .map(Arc::new);

    start_event_processor(&mut ebpf, tracer_v4.clone(), tracer_v6.clone())?;

    let state = Arc::new(AppState {
        tracer_v4,
        tracer_v6,
    });

    info!("Setting up server...");

    setup_server(&opt, state);

    info!("Server started. Press Ctrl+C to stop.");

    let ctrl_c = signal::ctrl_c();
    ctrl_c.await?;

    Ok(())
}
