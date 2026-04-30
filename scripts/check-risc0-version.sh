#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
LEZ_CARGO="$LEZ_REPO/Cargo.toml"
LOCAL_CARGO="$ROOT_DIR/Cargo.toml"

if [[ ! -f "$LEZ_CARGO" ]]; then
  echo "Missing LEZ Cargo.toml: $LEZ_CARGO" >&2
  exit 1
fi

lez_version="$(
  sed -n 's/^risc0-zkvm = { version = "\([^"]*\)".*/\1/p' "$LEZ_CARGO" | head -n 1
)"

if [[ -z "$lez_version" ]]; then
  echo "Could not detect risc0-zkvm version in $LEZ_CARGO" >&2
  exit 1
fi

echo "LEZ risc0-zkvm version: $lez_version"

if [[ ! -f "$LOCAL_CARGO" ]]; then
  echo "No attestation Cargo.toml yet; record this version for the first Rust milestone."
  exit 0
fi

local_version="$(
  sed -n 's/^risc0-zkvm = { version = "\([^"]*\)".*/\1/p' "$LOCAL_CARGO" | head -n 1
)"

if [[ -z "$local_version" ]]; then
  echo "No local risc0-zkvm dependency found yet; skipping workspace comparison."
  exit 0
fi

echo "Attestation risc0-zkvm version: $local_version"

if [[ "$lez_version" != "$local_version" ]]; then
  echo "RISC Zero version mismatch: LEZ=$lez_version attestation=$local_version" >&2
  exit 1
fi

echo "RISC Zero versions match."

