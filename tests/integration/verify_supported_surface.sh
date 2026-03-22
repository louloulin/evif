#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

echo "[verify] checking supported product paths are graph-free"
bash tests/integration/no_graph_deps.sh
bash tests/integration/no_graph_left.sh

echo "[verify] checking supported crates build"
cargo check -p evif-core -p evif-plugins -p evif-rest -p evif-cli -p evif-fuse -p evif-mcp

echo "[verify] checking focused regression suites"
cargo test -p evif-core --test plugin_lifecycle
cargo test -p evif-rest --test core_surface --test plugin_mount_contract --test memory_query_contract --test plugin_inventory_contract
cargo test -p evif-cli --test surface_contract
cargo test -p evif-plugins core_supported_plugins

if [ -d "evif-web/node_modules" ]; then
  echo "[verify] checking evif-web typecheck/build"
  (
    cd evif-web
    npm run verify
  )
else
  echo "[verify] skipping evif-web verify because node_modules is missing" >&2
fi
