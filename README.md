# inband-traceroute

Design is to use XDP on the interface to process ethernet packets. This avoids the need for a raw socket, etc.

Aya can create an XDP program template using cargo-generate: https://aya-rs.dev/book/start/development/#prerequisites

AsyncPerfEventArray (user mode) and PerfEventArray map for custom type.

array map can be used for configuration if required.

Example here:
https://github.com/aya-rs/book/blob/76fa86565f0f06f003536fb0cb496ab69f451ff6/examples/cgroup-skb-egress/cgroup-skb-egress-ebpf/src/main.rs

tokio for async support on user side
etherparse for packet parsing
rustls_acme https://docs.rs/rustls-acme/latest/rustls_acme/

https://docs.rs/etherparse/latest/etherparse/index.html

https://docs.rs/aya/latest/aya/maps/perf/struct.PerfEventArray.html

Hyper for HTTP support
Raw socket for sending packets..? (one for IPv4 and one for IPv6)



## Prerequisites

1. stable rust toolchains: `rustup toolchain install stable`
1. nightly rust toolchains: `rustup toolchain install nightly --component rust-src`
1. (if cross-compiling) rustup target: `rustup target add ${ARCH}-unknown-linux-musl`
1. (if cross-compiling) LLVM: (e.g.) `brew install llvm` (on macOS)
1. (if cross-compiling) C toolchain: (e.g.) [`brew install filosottile/musl-cross/musl-cross`](https://github.com/FiloSottile/homebrew-musl-cross) (on macOS)
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
