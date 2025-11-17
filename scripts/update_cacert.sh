#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
CERT_DIR="${ROOT_DIR}/certs"
mkdir -p "${CERT_DIR}"

curl -L "https://curl.se/ca/cacert.pem" -o "${CERT_DIR}/cacert.pem"
echo "Mozilla CA bundle synced to ${CERT_DIR}/cacert.pem"
