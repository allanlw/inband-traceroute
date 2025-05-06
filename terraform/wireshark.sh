#!/bin/bash

set -euo pipefail

wireshark_path="/mnt/c/Program Files/Wireshark/Wireshark.exe"

# Get dev_domain from terraform
dev_domain=$(terraform output -json | jq -r '.dev_domain.value')

ssh intr@$dev_domain "sudo tcpdump -w - -U 'not port 22'" | "$wireshark_path" -kS -i -
