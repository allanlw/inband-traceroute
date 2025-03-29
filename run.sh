#!/bin/bash

INTERFACE="ens4"
DOMAIN="inband-traceroute.net"

IPV4="$(ip addr show $INTERFACE | grep -oP '(?<=inet\s)\d+(\.\d+){3}')"
IPV6="$(ip addr show $INTERFACE | grep -oP '(?<=inet6\s)[0-9a-fA-F:]+(?=/)' | head -n 1)"

echo "IPv4: $IPV4"
echo "IPv6: $IPV6"

RUST_LOG=info cargo run --release --config 'target."cfg(all())".runner="sudo -E"' -- \
    --iface "$INTERFACE" \
    --domain "$DOMAIN" \
    --cache-dir ./.cert-cache \
    --address "$IPV4" \
    --address "$IPV6" \
    --prod