# Use the official Rust image as the base image
FROM rust:bullseye

# Set the working directory
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Build the dependencies
RUN cargo build --release

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release

# Set the entry point for the container
CMD ["./target/release/inband-traceroute"]
