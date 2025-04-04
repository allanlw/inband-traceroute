# Use the official Rust image as the base image
FROM rust:bullseye

RUN rustup toolchain install nightly --component rust-src && \
    cargo install bpf-linker

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY . .

# Build the dependencies
RUN cargo build --release

# Set the entry point for the container
CMD ["./target/release/inband-traceroute"]
