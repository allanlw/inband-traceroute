# inband-traceroute

This repository contains an HTTP server that implemented an inband traceroute functionality. This works by injecting redundant TCP keep alive packets into the 
stream for a HTTP2/TLS Server-Side Events (SSE) connection, and then relaying hop information in the response stream about incoming ICMP timeout packets.

A public instance is available at https://inband-traceroute.net/

Details:

- Backend implemented in Rust with eBPF (XDP) for incoming packet processing
    - eBPF support leveraging [aya](https://aya-rs.dev/)
- Supports IPv4/IPv6, TLS 1.3 (with ACME) and HTTP/2
    - Web layer uses [axum](https://github.com/tokio-rs/axum) (w/ [Tower](https://github.com/tower-rs/tower), [Hyper](https://hyper.rs/)) and [rusttls-acme](https://github.com/FlorianUekermann/rustls-acme)
- Traceroutes are enriched with [IPinfo Lite](https://ipinfo.io/lite) and reverse dns
    - maxmind db support via [maxminddb-rust](https://github.com/oschwald/maxminddb-rust)
    - DNS resolution via [hickory-dns](https://github.com/hickory-dns/hickory-dns)
- Static frontend in Typescript with [Vue.js](https://vuejs.org/)

Prior Art:

- [intrace](https://github.com/robertswiecki/intrace) by Robert Swiecki 
- [Collaborative Research Proposal: an In-Band Traceroute Service](https://www.ietf.org/proceedings/94/slides/slides-94-hopsrg-4.pdf) by Dave Plonka (Akamai)

## Build & Run

### Backend

Install the prerequisites:

1. stable rust toolchains: `rustup toolchain install stable`
1. nightly rust toolchains: `rustup toolchain install nightly --component rust-src`
1. bpf-linker: `cargo install bpf-linker` (`--no-default-features` on macOS)

From the `backend` directory, use `cargo build`, `cargo check`, etc. as normal. Run with:

```shell
cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

# License

Copyright (C) 2025 Allan Wirth

The ebpf portion is licensed under GPLv2, everything else (including the userspace portions and frontend) are licensed under AGPLv3.

This program incorporates software from [aya-template](https://github.com/aya-rs/aya-template) (see commit bbf949a2) which is used under the terms of the MIT license copyright 2021 Alessandro Decina.
