#!/bin/bash
# evif-verify.sh - RTK-style CLI wrapper for EVIF validation
#
# Usage: This script provides RTK-style meta commands for EVIF verification
#
# Meta Commands:
#   evif-verify run          # Run full validation suite
#   evif-verify quick        # Quick smoke test
#   evif-verify stats        # Show verification statistics
#   evif-verify history      # Show verification history
#   evif-verify discover     # Discover integration opportunities
#   evif-verify proxy <cmd>  # Execute raw command

EVIF_ROOT="${EVIF_ROOT:-/Users/louloulin/Documents/linchong/claude/evif}"
EVIF_SERVER="${EVIF_SERVER:-http://localhost:8080}"
OUTPUT_DIR="${EVIF_ROOT}/.claude"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check server health
check_server() {
    local health=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/api/v1/health" 2>/dev/null)
    if echo "$health" | grep -q "healthy"; then
        return 0
    else
        return 1
    fi
}

# Run osascript validation
run_applescript() {
    local script="$1"
    if [ -f "$script" ]; then
        osascript "$script" 2>/dev/null || echo "Failed to run AppleScript"
    else
        echo "Script not found: $script"
        return 1
    fi
}

# Meta command: stats
show_stats() {
    echo "═══════════════════════════════════════════════════"
    echo "EVIF Verification Statistics"
    echo "═══════════════════════════════════════════════════"
    echo ""

    # Check server status
    if check_server; then
        log_success "REST Server: healthy"
    else
        log_error "REST Server: not responding"
    fi

    echo ""

    # Metrics
    local metrics=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/metrics" 2>/dev/null)
    local metric_count=$(echo "$metrics" | grep -c "^evif_" 2>/dev/null || echo 0)
    echo "Prometheus metrics: $metric_count"

    # MCP tools
    local tools=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/api/v1/mcp/tools" 2>/dev/null)
    local tool_count=$(echo "$tools" | grep -o '"name"[[:space:]]*:' | wc -l | tr -d ' ')
    echo "MCP tools: $tool_count"

    echo ""

    # Skills
    local skill_count=$(ls "$EVIF_ROOT/.claude/skills"/*.SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    echo "Skills: $skill_count"

    echo ""

    # Verification history
    local output_file="${OUTPUT_DIR}/evif-verify-comprehensive-output.txt"
    if [ -f "$output_file" ]; then
        echo "Last verification:"
        tail -5 "$output_file" | head -3
        echo ""
        local timestamp_file="${OUTPUT_DIR}/evif-verify-timestamp.txt"
        if [ -f "$timestamp_file" ]; then
            cat "$timestamp_file"
        fi
    else
        log_warn "No verification history found"
    fi

    echo ""
}

# Meta command: history
show_history() {
    echo "═══════════════════════════════════════════════════"
    echo "EVIF Verification History"
    echo "═══════════════════════════════════════════════════"
    echo ""

    local output_file="${OUTPUT_DIR}/evif-verify-comprehensive-output.txt"
    if [ -f "$output_file" ]; then
        echo "Last 20 lines of verification:"
        tail -20 "$output_file"
    else
        log_warn "No verification history found"
        echo "Run 'evif-verify run' to perform first verification"
    fi

    echo ""
}

# Meta command: discover
discover_opportunities() {
    echo "═══════════════════════════════════════════════════"
    echo "EVIF Integration Opportunities"
    echo "═══════════════════════════════════════════════════"
    echo ""

    echo "Claude Code Integration Opportunities:"
    echo ""

    # Check MCP capabilities
    local mcp_health=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/api/v1/mcp/health" 2>/dev/null)
    if echo "$mcp_health" | grep -q "healthy"; then
        log_success "MCP HTTP mode: available"
        echo "  - 18+ tools via unified API"
        echo "  - Token optimization (60-90% savings)"
        echo "  - Cross-session memory"
    else
        log_warn "MCP HTTP mode: not available"
    fi

    echo ""

    # Check context layers
    local ctx=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/api/v1/directories?path=/context" 2>/dev/null)
    if echo "$ctx" | grep -q "L0" && echo "$ctx" | grep -q "L1" && echo "$ctx" | grep -q "L2"; then
        log_success "Context layers (L0/L1/L2): available"
        echo "  - Cross-session memory"
        echo "  - Decision tracking"
        echo "  - Project knowledge persistence"
    else
        log_warn "Context layers: partial"
    fi

    echo ""

    # Check skills
    local skill_count=$(ls "$EVIF_ROOT/.claude/skills"/*.SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    if [ "$skill_count" -gt 0 ]; then
        log_success "Skills system: $skill_count skills"
        echo "  - Reusable workflows"
        echo "  - Multi-agent coordination"
    else
        log_warn "Skills system: not initialized"
    fi

    echo ""

    # Check RTK-style wrapper
    if [ -f "$EVIF_ROOT/scripts/oscript/evif-proxy.sh" ]; then
        log_success "RTK-style wrapper: available"
        echo "  - Transparent CLI integration"
        echo "  - Meta commands (gain, discover, proxy)"
    else
        log_warn "RTK-style wrapper: not found"
    fi

    echo ""
}

# Meta command: quick
run_quick() {
    echo "═══════════════════════════════════════════════════"
    echo "EVIF Quick Smoke Test"
    echo "═══════════════════════════════════════════════════"
    echo ""

    # Check prerequisites
    log_info "Checking prerequisites..."

    for cmd in curl osascript; do
        if command -v "$cmd" &> /dev/null; then
            log_success "$cmd: available"
        else
            log_error "$cmd: not found"
        fi
    done

    echo ""

    # Check server
    log_info "Checking server..."

    if check_server; then
        log_success "REST Server: healthy"

        # Quick MCP check
        local mcp_tools=$(curl -s --max-time 5 --noproxy '*' "${EVIF_SERVER}/api/v1/mcp/tools" 2>/dev/null)
        if echo "$mcp_tools" | grep -q "evif_ls"; then
            log_success "MCP tools: available"
        else
            log_warn "MCP tools: limited"
        fi
    else
        log_error "REST Server: not responding"
        log_info "Start server: cargo run -p evif-rest"
    fi

    echo ""

    # Check skills
    local skill_count=$(ls "$EVIF_ROOT/.claude/skills"/*.SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    if [ "$skill_count" -gt 0 ]; then
        log_success "Skills: $skill_count found"
    else
        log_warn "Skills: none found"
    fi

    echo ""
    echo "Quick test complete!"
}

# Meta command: run
run_full() {
    echo "═══════════════════════════════════════════════════"
    echo "EVIF Comprehensive Validation"
    echo "═══════════════════════════════════════════════════"
    echo ""
    log_info "Starting full validation suite..."
    echo ""

    # Check if server is running
    if ! check_server; then
        log_error "Server not running. Starting server..."
        (cd "$EVIF_ROOT" && cargo run -p evif-rest &
         sleep 5)
        if ! check_server; then
            log_error "Failed to start server. Please start manually:"
            echo "  cd $EVIF_ROOT && cargo run -p evif-rest"
            exit 1
        fi
    fi

    log_success "Server is running"
    echo ""

    # Run comprehensive AppleScript validation
    log_info "Running AppleScript validation..."
    echo ""

    local script="$EVIF_ROOT/scripts/oscript/evif-verify-comprehensive.scpt"
    run_applescript "$script"

    echo ""
    log_success "Validation complete!"
    log_info "Results saved to: $OUTPUT_DIR/evif-verify-comprehensive-output.txt"
}

# Main command parser
case "${1:-}" in
    run)
        run_full
        ;;
    quick)
        run_quick
        ;;
    stats)
        show_stats
        ;;
    history)
        show_history
        ;;
    discover)
        discover_opportunities
        ;;
    proxy)
        shift
        # Execute raw command via osascript
        if [ -n "$1" ]; then
            osascript -e "do shell script \"$*\""
        else
            log_error "Usage: evif-verify proxy <command>"
        fi
        ;;
    --help|-h)
        echo "EVIF Verification - RTK-style CLI wrapper"
        echo ""
        echo "Usage: evif-verify <command>"
        echo ""
        echo "Commands:"
        echo "  run          Run full validation suite"
        echo "  quick        Quick smoke test"
        echo "  stats        Show verification statistics"
        echo "  history      Show verification history"
        echo "  discover     Discover integration opportunities"
        echo "  proxy <cmd>  Execute raw command"
        echo "  --help       Show this help"
        echo ""
        ;;
    *)
        if [ -z "$1" ]; then
            # Default: run quick
            run_quick
        else
            log_error "Unknown command: $1"
            echo "Use 'evif-verify --help' for usage"
            exit 1
        fi
        ;;
esac