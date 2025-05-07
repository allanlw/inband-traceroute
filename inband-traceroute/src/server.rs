use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use async_stream::try_stream;
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, State},
    response::Response,
    routing::get,
    Router,
};
use log::{error, info};
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio_stream::StreamExt;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

use crate::tracer::TraceHandle;

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) tracer_v4: Option<Arc<crate::tracer::Tracer>>,
    pub(crate) tracer_v6: Option<Arc<crate::tracer::Tracer>>,
}

// TODO: Fix panics
pub(crate) async fn index_handler(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> Response {
    let tracer = match remote {
        SocketAddr::V4(_) => state.tracer_v4.clone(),
        SocketAddr::V6(_) => state.tracer_v6.clone(),
    }
    .expect("If we got a connection in this protocol, the program should have a tracer for it");

    info!("Remote: {remote:?}");

    let trace_handle = TraceHandle::start_trace(tracer, remote).await.unwrap();

    let stream = try_stream! {
        let mut hop_stream = Box::pin(trace_handle.hop_stream().await.unwrap());
        while let Some(hop) = hop_stream.next().await {
            let hop = format!("{hop}\n");
            yield hop.into();
            yield Bytes::from_static(b"<br>\n");
        }
    };

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=UTF-8")
        .body(Body::from_stream(stream.map(
            |b: anyhow::Result<Bytes>| -> anyhow::Result<Bytes> { b },
        )))
        .unwrap()
}

pub(crate) fn setup_server(opt: &crate::Opt, state: Arc<AppState>) {
    let mut acme_state = AcmeConfig::new(vec![opt.domain.clone()])
        .contact(opt.emails.iter().map(|e| format!("mailto:{e}")))
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
                Ok(ok) => info!("ACME event: {ok:?}"),
                Err(err) => error!("ACME error: {err:?}"),
            }
        }
    });

    let addresses: Vec<SocketAddr> = [opt.ipv4.map(IpAddr::V4), opt.ipv6.map(IpAddr::V6)]
        .into_iter()
        .filter_map(|ip| ip.map(|ip| SocketAddr::new(ip, opt.port)))
        .collect();

    for addr in addresses {
        info!("Listening on {addr}");
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
