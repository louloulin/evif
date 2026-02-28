#!/usr/bin/env bash
# EVIF REST Server Manager
# Functions for starting/stopping EVIF REST server during tests

set -euo pipefail

# Server configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
export EVIF_SERVER_HOST="${EVIF_SERVER_HOST:-localhost}"
export EVIF_SERVER_PID=""

# Start REST server
start_rest_server() {
    local port="${1:-$EVIF_SERVER_PORT}"

    log_info "Starting EVIF REST server on port $port..."

    # Check if server already running
    if check_server "$port" 0; then
        log_info "Server already running on port $port"
        return 0
    fi

    # Start server in background
    cargo run -p evif-rest -- --port "$port" > /tmp/evif_server_$$.log 2>&1 &
    EVIF_SERVER_PID=$!

    log_info "Server started with PID $EVIF_SERVER_PID"

    # Wait for server to be ready
    log_info "Waiting for server to be ready..."
    local max_wait=30
    local waited=0

    while [ $waited -lt $max_wait ]; do
        if check_server "$port" 1; then
            log_info "Server is ready!"
            return 0
        fi
        sleep 1
        ((waited++))
    done

    log_fail "Server failed to start within ${max_wait}s" "server ready" "timeout"
    stop_rest_server
    return 1
}

# Stop REST server
stop_rest_server() {
    if [ -n "${EVIF_SERVER_PID:-}" ]; then
        log_info "Stopping REST server (PID: $EVIF_SERVER_PID)..."
        kill "$EVIF_SERVER_PID" 2>/dev/null || true
        wait "$EVIF_SERVER_PID" 2>/dev/null || true
        EVIF_SERVER_PID=""
        log_info "Server stopped"
    fi
}

# Get server status
get_server_status() {
    local port="${1:-$EVIF_SERVER_PORT}"

    if curl -s "http://localhost:${port}/health" 2>/dev/null; then
        return 0
    fi
    return 1
}

# Check if server is ready
check_server() {
    local port="${1:-$EVIF_SERVER_PORT}"
    local wait_seconds="${2:-1}"

    if curl -s "http://localhost:${port}/health" >/dev/null 2>&1; then
        return 0
    fi

    if [ "$wait_seconds" -gt 0 ]; then
        sleep "$wait_seconds"
        return check_server "$port" 0
    fi

    return 1
}

# Cleanup on exit
cleanup_server() {
    log_info "Cleaning up server..."
    stop_rest_server
}

trap cleanup_server EXIT

# Health check helper
health_check() {
    local port="${1:-$EVIF_SERVER_PORT}"
    local response

    response=$(curl -s "http://localhost:${port}/health" 2>/dev/null || echo "")

    if [ -n "$response" ]; then
        echo "$response"
        return 0
    fi

    return 1
}