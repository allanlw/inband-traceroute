use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    ops::DerefMut,
    sync::Arc,
};

use async_stream::{stream, try_stream};
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, State},
    http::HeaderValue,
    response::{sse::Event, Response, Sse},
    routing::get,
    Router,
};
use futures::{Stream, TryStream};
use http::request::Parts as RequestParts;
use hyper::Method;
use log::{error, info};
use rustls_acme::{caches::DirCache, AcmeConfig};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{self, TraceLayer},
};
use tracing::Level;

use crate::tracer::TraceHandle;

#[derive(serde::Serialize, Debug)]
pub enum TraceEvent {
    Hop(crate::hop::Hop),
    ReverseDns { ip: IpAddr, name: String },
}

#[derive(Debug)]
pub(crate) struct AppState {
    pub(crate) tracer_v4: Option<Arc<crate::tracer::Tracer>>,
    pub(crate) tracer_v6: Option<Arc<crate::tracer::Tracer>>,
}

impl AppState {
    fn get_tracer(&self, remote: SocketAddr) -> Arc<crate::tracer::Tracer> {
        match remote {
            SocketAddr::V4(_) => self.tracer_v4.clone(),
            SocketAddr::V6(_) => self.tracer_v6.clone(),
        }
        .expect("If we got a connection in this protocol, the program should have a tracer for it")
    }

    async fn trace_stream(
        &self,
        remote: SocketAddr,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<TraceEvent>>> {
        let tracer = self.get_tracer(remote);

        info!("Remote: {remote:?}");

        let trace_handle = TraceHandle::start_trace(tracer.clone(), remote).await?;

        // channels automatically close when all senders are dropped
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<anyhow::Result<TraceEvent>>();

        tokio::spawn(async move {
            let mut hop_stream = Box::pin(trace_handle.hop_stream().await.unwrap());
            while let Some(hop) = hop_stream.next().await {
                let addr = hop.addr;
                tx.send(Ok(TraceEvent::Hop(hop))).unwrap();
                if let Some(ip) = addr {
                    let tx = tx.clone();
                    let tracer = tracer.clone();
                    tokio::spawn(async move {
                        tx.send(
                            tracer
                                .dns_client
                                .reverse_lookup(&ip)
                                .await
                                .map(|name| TraceEvent::ReverseDns { ip, name }),
                        )
                        .unwrap();
                    });
                }
            }
        });

        Ok(UnboundedReceiverStream::new(rx))
    }
}

// TODO: Fix panics
async fn index_handler(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> Response {
    let tracer = state.get_tracer(remote);

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

async fn sse_handler(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> Sse<impl Stream<Item = anyhow::Result<Event>>> {
    let stream_result = state.trace_stream(remote).await.unwrap();

    Sse::new(stream_result.map(|event| -> anyhow::Result<Event> {
        let event = event?;
        let res = Event::default().json_data(event)?;
        Ok(res)
    }))
}

pub(crate) fn setup_server(opt: &crate::Opt, state: Arc<AppState>) {
    let mut domains = vec![opt.domain.clone()];
    if opt.v4_v6_subdomains {
        domains.push("ipv4.".to_owned() + &opt.domain);
        domains.push("ipv6.".to_owned() + &opt.domain);
    }

    let mut acme_state = AcmeConfig::new(domains)
        .contact(opt.emails.iter().map(|e| format!("mailto:{e}")))
        .cache_option(opt.cache_dir.clone().map(DirCache::new))
        .directory_lets_encrypt(opt.prod)
        .state();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            |origin: &HeaderValue, _request_parts: &RequestParts| {
                let origin = origin.as_bytes();
                // Allow localhost with any port
                if origin.starts_with(b"http://localhost:") {
                    return true;
                }
                // Allow inband-traceroute.net and subdomains
                if origin.ends_with(b".inband-traceroute.net")
                    || origin == b"https://inband-traceroute.net"
                {
                    return true;
                }
                false
            },
        ))
        .allow_methods([Method::GET]);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/sse", get(sse_handler))
        .with_state(state)
        .layer(cors)
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
