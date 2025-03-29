# inband-traceroute

AsyncPerfEventArray (user mode) and PerfEventArray map for custom type.

array map can be used for configuration if required.

Example here:
https://github.com/aya-rs/book/blob/76fa86565f0f06f003536fb0cb496ab69f451ff6/examples/cgroup-skb-egress/cgroup-skb-egress-ebpf/src/main.rs

https://docs.rs/aya/latest/aya/maps/perf/struct.PerfEventArray.html

Raw socket for sending packets..? (one for IPv4 and one for IPv6)



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
