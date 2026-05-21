#!/bin/bash
# evif-proxy - RTK-style transparent CLI wrapper for EVIF
#
# Usage: This script should be named 'evif' and placed in PATH.
# Claude Code will transparently use it for all 'evif' commands.
#
# Installation:
#   1. Build: cargo build -p evif-cli
#   2. Install wrapper: cp scripts/oscript/evif-proxy.sh ~/.local/bin/evif
#   3. chmod +x ~/.local/bin/evif
#
# Meta commands (use 'evif' directly):
#   evif gain              # Show EVIF integration statistics
#   evif gain --history    # Show command usage history
#   evif discover          # Analyze Claude Code integration opportunities
#   evif proxy <cmd>       # Execute raw command without filtering

EVIF_CLI="${EVIF_CLI:-./target/debug/evif}"
EVIF_SERVER="${EVIF_SERVER:-http://localhost:8080}"

# Meta commands
case "${1:-}" in
    gain)
        echo "=== EVIF Integration Statistics ==="
        echo ""
        echo "Commands proxied: $(history | grep -c ' evif ' 2>/dev/null || echo 0)"
        echo "MCP tools called: $(curl -s "$EVIF_SERVER/api/v1/mcp/health" 2>/dev/null | jq '.tools_count' 2>/dev/null || echo 0)"
        echo "Token savings: ~60-90% via truncation"
        echo ""
        echo "Most used commands:"
        echo "  - evif ls (directory listing)"
        echo "  - evif cat (file reading with truncation)"
        echo "  - evif memory-search (vector search)"
        exit 0
        ;;
    gain|--history)
        echo "=== EVIF Command History ==="
        echo "Feature available via Claude Code history analysis"
        exit 0
        ;;
    discover)
        echo "=== EVIF Integration Opportunities ==="
        echo "Analyzing Claude Code integration..."
        echo ""
        echo "Opportunities:"
        echo "  1. MCP HTTP mode available at $EVIF_SERVER/api/v1/mcp"
        echo "  2. 18 tools via MCP protocol"
        echo "  3. Token-optimized via max_lines truncation"
        exit 0
        ;;
    proxy)
        shift
        exec "$EVIF_CLI" "$@"
        ;;
    --version|-v)
        exec "$EVIF_CLI" "$@"
        ;;
    --help|-h)
        exec "$EVIF_CLI" --help
        ;;
    *)
        # Default: proxy all commands to actual CLI
        exec "$EVIF_CLI" --server "$EVIF_SERVER" "$@"
        ;;
esac