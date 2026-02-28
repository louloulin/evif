#!/usr/bin/env bash
# EVIF Performance Benchmarks
# Tests for API latency, file throughput, and server startup time

set -euo pipefail

# Source test libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test_helpers.sh"
source "$SCRIPT_DIR/../lib/assertions.sh"
source "$SCRIPT_DIR/../lib/server_manager.sh"

# Test configuration
export EVIF_SERVER_PORT="${EVIF_SERVER_PORT:-8080}"
BASE_URL="http://localhost:${EVIF_SERVER_PORT}"

log_section "Performance Benchmarks"

# Timing helper
time_cmd() {
    local start end elapsed
    start=$(date +%s.%N)
    "$@"
    local ret=$?
    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    echo "$elapsed"
    return $ret
}

# API Latency Tests
test_api_latency() {
    log_info "=== API Latency Tests ==="

    # Warmup
    curl -s "${BASE_URL}/health" > /dev/null || return 1

    # Measure 10 requests
    local times=()
    for i in {1..10}; do
        times+=($(time_cmd curl -s "${BASE_URL}/health" > /dev/null))
    done

    # Sort for percentile calculation
    IFS=$'\n' sorted=($(sort -n <<<"${times[*]}"))
    unset IFS

    local p50="${sorted[4]}"
    local p95="${sorted[8]}"
    local p99="${sorted[9]}"

    echo "P50: ${p50}s"
    echo "P95: ${p95}s"
    echo "P99: ${p99}s"

    # PASS/FAIL criteria - P99 must be < 0.1s (100ms)
    if (( $(echo "$p99 < 0.1" | bc -l) )); then
        log_pass "API P99 latency ${p99}s < 100ms"
        return 0
    else
        log_fail "API P99 latency" "< 100ms" "${p99}s >= 100ms"
        return 1
    fi
}

# File Operation Throughput
test_file_throughput() {
    log_info "=== File Operation Throughput ==="

    local count=100
    local start end ops_per_sec

    start=$(date +%s)
    for i in $(seq 1 $count); do
        curl -s -X POST "${BASE_URL}/api/v1/files" \
            -H "Content-Type: application/json" \
            -d "{\"path\":\"/perf/test${i}.txt\",\"content\":\"test\"}" > /dev/null || true
    done
    end=$(date +%s)

    local elapsed=$((end - start))
    ops_per_sec=$(echo "scale=2; $count / $elapsed" | bc)
    echo "Throughput: $ops_per_sec ops/sec (elapsed: ${elapsed}s)"

    # Cleanup
    for i in $(seq 1 $count); do
        curl -s -X DELETE "${BASE_URL}/api/v1/files?path=/perf/test${i}.txt" > /dev/null || true
    done

    # PASS/FAIL criteria - must achieve > 10 ops/sec
    if (( $(echo "$ops_per_sec > 10" | bc -l) )); then
        log_pass "Throughput ${ops_per_sec} ops/sec > 10 ops/sec"
        return 0
    else
        log_fail "Throughput" "> 10 ops/sec" "${ops_per_sec} ops/sec <= 10"
        return 1
    fi
}

# Server Startup Time
test_server_startup() {
    log_info "=== Server Startup Time ==="

    # Stop any existing server
    stop_rest_server 2>/dev/null || true
    sleep 1

    local start end elapsed
    start=$(date +%s.%N)

    # Start server on different port
    cargo run -p evif-rest -- --port 8081 > /tmp/evif_startup.log 2>&1 &
    local pid=$!

    while ! curl -s http://localhost:8081/health > /dev/null 2>&1; do
        sleep 0.5
        if ! kill -0 $pid 2>/dev/null; then
            log_fail "Server failed to start" "server ready" "process died"
            return 1
        fi
    done

    end=$(date +%s.%N)
    elapsed=$(echo "$end - $start" | bc)
    echo "Startup time: ${elapsed}s"

    # Cleanup
    kill $pid 2>/dev/null || true

    # PASS/FAIL criteria - must start within 30 seconds
    if (( $(echo "$elapsed < 30" | bc -l) )); then
        log_pass "Server startup ${elapsed}s < 30s"
        return 0
    else
        log_fail "Server startup" "< 30s" "${elapsed}s >= 30s"
        return 1
    fi
}

# Test Suite Runtime
test_suite_runtime() {
    log_info "=== Test Suite Runtime ==="

    local start end elapsed
    start=$(date +%s)

    # Run a subset of tests to measure
    "$SCRIPT_DIR/../integration/cli/file_operations.sh" >/dev/null 2>&1 || true

    end=$(date +%s)
    elapsed=$((end - start))
    echo "Subset runtime: ${elapsed}s"

    # PASS/FAIL criteria - full suite should run in < 5 minutes
    # Using subset as proxy (should be much faster)
    if [ $elapsed -lt 60 ]; then
        log_pass "Test subset runtime ${elapsed}s < 60s (full suite target: <5m)"
        return 0
    else
        log_fail "Test runtime" "< 60s for subset" "${elapsed}s"
        return 1
    fi
}

# Run all benchmarks
main() {
    log_info "Starting Performance Benchmarks..."

    # Start server for benchmarks that need it
    start_rest_server "$EVIF_SERVER_PORT" || {
        log_error "Failed to start REST server for benchmarks"
        exit 1
    }

    run_test "API P99 Latency" test_api_latency
    run_test "File Write Throughput" test_file_throughput
    run_test "Server Startup Time" test_server_startup
    run_test "Test Suite Runtime" test_suite_runtime

    print_test_summary
}

main "$@"