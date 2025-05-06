# inband-traceroute

This repository contains an HTTP server that implemented an inband traceroute functionality.

Prior Art:
- [intrace](https://github.com/robertswiecki/intrace) by Robert Swiecki 
- [Collaborative Research Proposal: an In-Band Traceroute Service](https://www.ietf.org/proceedings/94/slides/slides-94-hopsrg-4.pdf) by Dave Plonka (Akamai)


Details:

- Implemented in Rust with eBPF (XDP) for packet processing
- Supports TLS 1.3 and HTTP2

## Prerequisites

1. stable rust toolchains: `rustup toolchain install stable`
1. nightly rust toolchains: `rustup toolchain install nightly --component rust-src`
1. bpf-linker: `cargo install bpf-linker` (`--no-default-features` on macOS)

## Build & Run

Use `cargo build`, `cargo check`, etc. as normal. Run your program with:

```shell
cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

Cargo build scripts are used to automatically build the eBPF correctly and include it in the
program.

## Cross-compiling on macOS

Cross compilation should work on both Intel and Apple Silicon Macs.

```shell
CC=${ARCH}-linux-musl-gcc cargo build --package inband-traceroute --release \
  --target=${ARCH}-unknown-linux-musl \
  --config=target.${ARCH}-unknown-linux-musl.linker=\"${ARCH}-linux-musl-gcc\"
```
The cross-compiled program `target/${ARCH}-unknown-linux-musl/release/inband-traceroute` can be
copied to a Linux server or VM and run there.

# License

Copyright (C) 2025 Allan Wirth

The kernel portion is licensed under GPLv2, everything else (including the userspace portions) are licensed under AGPLv3. 

This program incorporates software from [aya-template](https://github.com/aya-rs/aya-template) (see commit bbf949a2) which is used under the terms of the MIT license copyright 2021 Alessandro Decina.