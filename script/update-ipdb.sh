#!/bin/bash

set -euo pipefail

TOKEN="$(cat /ipinfoio_token)"
OUTPUT_FILE="/opt/ipinfoio/ipinfo_lite.mmdb"

if [ -f "$OUTPUT_FILE" ]; then
    # Check if file is older than 24 hours (86400 seconds)
    if [ $(($(date +%s) - $(stat -c %Y "$OUTPUT_FILE"))) -lt 86400 ]; then
        echo "File is up to date. Skipping download."
        exit 0
    fi
fi

curl -L https://ipinfo.io/data/ipinfo_lite.mmdb?token="${TOKEN}" \
  -o "$OUTPUT_FILE"
