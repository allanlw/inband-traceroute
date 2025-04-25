#!/bin/bash

set -euxo pipefail

rustup toolchain install stable
rustup toolchain install nightly --component rust-src
cargo install bpf-linker

