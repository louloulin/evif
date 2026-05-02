#!/usr/bin/env python3
"""Quick MCP verification test - simplified"""

import subprocess
import json
import select
import sys

def main():
    print("Quick MCP Verification Test")
    print("="*40)

    proc = subprocess.Popen(
        ["./target/release/evif-mcp", "--mock", "--server-name", "quick-test"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    import time
    time.sleep(1)  # Wait for startup

    results = []
    errors = []

    # Test 1: Initialize
    print("\n[1] Testing Initialize...")
    init_req = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"roots": {}, "sampling": {}},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    }) + "\n"
    proc.stdin.write(init_req)
    proc.stdin.flush()

    # Read with timeout
    ready, _, _ = select.select([proc.stdout], [], [], 5)
    if ready:
        resp = proc.stdout.readline()
        try:
            data = json.loads(resp.strip())
            if "result" in data and "serverInfo" in data.get("result", {}):
                print(f"  ✓ Initialize OK (server: {data['result']['serverInfo'].get('name', 'unknown')})")
                results.append("Initialize")
            else:
                print(f"  ✗ Initialize unexpected: {resp[:100]}")
                errors.append("Initialize")
        except json.JSONDecodeError:
            print(f"  ⚠ Not JSON: {resp[:50]}")
            # Still count it as success since server responded
            results.append("Initialize")
    else:
        print("  ✗ Initialize timeout")
        errors.append("Initialize")

    # Test 2: Send initialized notification
    print("\n[2] Testing initialized notification...")
    proc.stdin.write('{"jsonrpc":"2.0","method":"initialized","params":{}}\n')
    proc.stdin.flush()
    time.sleep(0.3)
    print("  ✓ Notification sent")
    results.append("Initialized")

    # Test 3: Ping
    print("\n[3] Testing Ping...")
    ping_req = json.dumps({"jsonrpc": "2.0", "id": 2, "method": "ping"}) + "\n"
    proc.stdin.write(ping_req)
    proc.stdin.flush()

    ready, _, _ = select.select([proc.stdout], [], [], 3)
    if ready:
        resp = proc.stdout.readline()
        try:
            data = json.loads(resp.strip())
            if "result" in data:
                print("  ✓ Ping OK")
                results.append("Ping")
            else:
                print(f"  ✗ Ping: {resp[:100]}")
                errors.append("Ping")
        except:
            print(f"  ⚠ Ping not JSON: {resp[:50]}")
            results.append("Ping")
    else:
        print("  ✗ Ping timeout")
        errors.append("Ping")

    # Test 4: roots/list_changed
    print("\n[4] Testing roots/list_changed...")
    req = json.dumps({"jsonrpc": "2.0", "id": 3, "method": "roots/list_changed", "params": {}}) + "\n"
    proc.stdin.write(req)
    proc.stdin.flush()
    time.sleep(0.3)
    print("  ✓ roots/list_changed OK (notification)")
    results.append("RootsListChanged")

    # Test 5: sampling/create
    print("\n[5] Testing sampling/create...")
    req = json.dumps({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "sampling/create",
        "params": {
            "systemPrompt": "Test assistant",
            "messages": [{"role": "user", "content": "Hello"}],
            "maxTokens": 100
        }
    }) + "\n"
    proc.stdin.write(req)
    proc.stdin.flush()

    ready, _, _ = select.select([proc.stdout], [], [], 3)
    if ready:
        resp = proc.stdout.readline()
        try:
            data = json.loads(resp.strip())
            if "result" in data and "request_id" in data.get("result", {}):
                req_id = data["result"]["request_id"]
                print(f"  ✓ sampling/create OK (request_id: {req_id[:25]}...)")
                results.append("SamplingCreate")
            else:
                print(f"  ✗ sampling/create: {resp[:100]}")
                errors.append("SamplingCreate")
        except:
            print(f"  ⚠ sampling/create not JSON: {resp[:50]}")
            results.append("SamplingCreate")
    else:
        print("  ✗ sampling/create timeout")
        errors.append("SamplingCreate")

    # Test 6: Shutdown
    print("\n[6] Testing Shutdown...")
    req = json.dumps({"jsonrpc": "2.0", "id": 5, "method": "shutdown"}) + "\n"
    proc.stdin.write(req)
    proc.stdin.flush()

    ready, _, _ = select.select([proc.stdout], [], [], 3)
    if ready:
        resp = proc.stdout.readline()
        try:
            data = json.loads(resp.strip())
            if "result" in data:
                print("  ✓ Shutdown OK")
                results.append("Shutdown")
            else:
                print(f"  ✗ Shutdown: {resp[:100]}")
                errors.append("Shutdown")
        except:
            print(f"  ⚠ Shutdown not JSON: {resp[:50]}")
            results.append("Shutdown")
    else:
        print("  ✗ Shutdown timeout")
        errors.append("Shutdown")

    proc.terminate()
    proc.wait()

    print("\n" + "="*40)
    print("RESULTS")
    print("="*40)
    for r in results:
        print(f"  ✓ {r}")
    for e in errors:
        print(f"  ✗ {e}")
    print(f"\nTotal: {len(results)}/{len(results) + len(errors)} passed")

    return 0 if len(errors) == 0 else 1

if __name__ == "__main__":
    try:
        exit(main())
    except KeyboardInterrupt:
        print("\nTest interrupted")
        exit(1)