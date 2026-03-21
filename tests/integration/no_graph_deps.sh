#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

paths=(
  "crates/evif-rest/Cargo.toml"
  "crates/evif-cli/Cargo.toml"
  "crates/evif-fuse/Cargo.toml"
  "crates/evif-rest/src"
  "crates/evif-cli/src"
  "crates/evif-fuse/src"
)

if rg -n "evif-graph" "${paths[@]}"; then
  echo "error: direct evif-graph references still exist in supported product paths" >&2
  exit 1
fi

echo "ok: no direct evif-graph references in supported product paths"
