#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

paths=(
  "crates"
  "tests"
  "examples"
  "evif-web/src"
)

pattern_parts=("\\bevif_graph\\b" "\\bGraph::new\\b" "\\bNodeType\\b" "\\bNodeId\\b")
pattern=$(IFS='|'; echo "${pattern_parts[*]}")

if rg -g '!tests/integration/no_graph_left.sh' -n "$pattern" "${paths[@]}"; then
  echo "error: graph-era symbols still exist in supported paths" >&2
  exit 1
fi

echo "ok: no graph-era symbols remain in supported paths"
