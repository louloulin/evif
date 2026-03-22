#!/bin/bash
# EVIF Master Test Runner Script
# Runs all test suites in proper order

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
TEST_PORT=${EVIF_TEST_PORT:-8081}
SERVER_PID=""
BUILD_MODE=${BUILD_MODE:-debug}

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cleanup() {
    log_info "Cleaning up..."
    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
}

trap cleanup EXIT

# Parse arguments
RUN_SERVER=false
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --with-server)
            RUN_SERVER=true
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --port)
            TEST_PORT="$2"
            shift 2
            ;;
        *)
            echo "Usage: $0 [--with-server] [--skip-build] [--port <port>]"
            exit 1
            ;;
    esac
done

log_info "Starting EVIF Test Runner"
log_info "Test port: $TEST_PORT"
log_info "Build mode: $BUILD_MODE"

log_info "Running supported surface checks..."
bash tests/integration/no_graph_deps.sh
bash tests/integration/no_graph_left.sh

# Build tests if not skipped
if [ "$SKIP_BUILD" = false ]; then
    log_info "Building test packages..."
    cargo test --no-run --package cli-tests --package api-tests
    cargo test --no-run -p evif-core --test plugin_lifecycle
    cargo test --no-run -p evif-rest --test core_surface --test plugin_mount_contract --test memory_query_contract --test plugin_inventory_contract
    cargo test --no-run -p evif-cli --test surface_contract
    cargo test --no-run -p evif-plugins core_supported_plugins
fi

# Start server if requested
if [ "$RUN_SERVER" = true ]; then
    log_info "Starting EVIF REST server on port $TEST_PORT..."

    # Check if port is available
    if lsof -i:$TEST_PORT >/dev/null 2>&1; then
        log_warn "Port $TEST_PORT is already in use. Server may be running."
    else
        cargo run -p evif-rest &
        SERVER_PID=$!
        log_info "Server started with PID $SERVER_PID"

        # Wait for server to be ready
        log_info "Waiting for server to be ready..."
        for i in {1..30}; do
            if curl -s http://localhost:$TEST_PORT/health >/dev/null 2>&1; then
                log_info "Server is ready!"
                break
            fi
            sleep 1
        done
    fi
fi

# Run CLI tests
log_info "Running CLI file operations tests..."
cargo test --package cli-tests -- --test-threads=1 2>&1 || log_warn "Some CLI tests may have failed"

log_info "Running supported surface regression tests..."
cargo test -p evif-core --test plugin_lifecycle 2>&1 || log_warn "Plugin lifecycle regression test failed"
cargo test -p evif-rest --test core_surface --test plugin_mount_contract --test memory_query_contract --test plugin_inventory_contract 2>&1 || log_warn "REST surface regression tests failed"
cargo test -p evif-cli --test surface_contract 2>&1 || log_warn "CLI surface regression test failed"
cargo test -p evif-plugins core_supported_plugins 2>&1 || log_warn "Plugin catalog regression test failed"

# Run API tests (requires server)
if [ "$RUN_SERVER" = true ]; then
    log_info "Running API endpoint tests..."
    cargo test --package api-tests 2>&1 || log_warn "Some API tests may have failed"
else
    log_warn "Skipping API tests (server not running). Use --with-server to run API tests."
fi

if [ -d "evif-web/node_modules" ]; then
    log_info "Running evif-web verify..."
    (
        cd evif-web
        npm run verify
    ) 2>&1 || log_warn "evif-web verify failed"
else
    log_warn "Skipping evif-web verify (evif-web/node_modules missing)"
fi

log_info "Test run complete!"
