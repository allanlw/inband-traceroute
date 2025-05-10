#!/bin/bash

if ! command -v cargo >/dev/null 2>&1; then
  source /opt/rust/env
fi

INTERFACE="$(ip -o link show | awk -F': ' '{print $2}' | grep -v lo | head -n 1)"
if [ -z "$INTERFACE" ]; then
    echo "No network interface found."
    exit 1
fi
echo "Using network interface: $INTERFACE"
DOMAIN="$(hostname | tr '0' '.')"
if [ -z "$DOMAIN" ]; then
    echo "Failed to resolve domain."
    exit 1
fi
echo "Resolved domain: $DOMAIN"

IPV4="$(ip addr show $INTERFACE | grep -oP '(?<=inet\s)\d+(\.\d+){3}')"
IPV6="$(ip addr show $INTERFACE | grep -oP '(?<=inet6\s)[0-9a-fA-F:]+(?=/)' | head -n 1)"

echo "IPv4: $IPV4"
echo "IPv6: $IPV6"

export RUST_BACKTRACE=1
export RUST_LOG=info

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd -P)"

cd "$script_dir/../backend"

cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- \
    --iface "$INTERFACE" \
    --domain "$DOMAIN" \
    --cache-dir ./.cert-cache \
    --ipv4 "$IPV4" \
    --ipv6 "$IPV6" \
    --prod
