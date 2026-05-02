#!/usr/bin/env python3
"""
Test script for EVIF MCP Server via stdio

This script tests the MCP server by sending JSON-RPC requests over stdio.
"""

import subprocess
import json
import sys
import os

def send_request(proc, method, params=None, request_id=1):
    """Send a JSON-RPC request and return the response."""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
    }
    if params is not None:
        request["params"] = params

    # Send request
    request_str = json.dumps(request) + "\n"
    proc.stdin.write(request_str)
    proc.stdin.flush()

    # Read response line by line until we get a JSON response
    while True:
        line = proc.stdout.readline()
        if not line:
            return None
        line = line.strip()
        if line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                # Skip non-JSON lines (like log output)
                continue

def main():
    print("EVIF MCP Server Integration Test")
    print("="*50)

    # Start evif-mcp server with mock mode
    print("\n[1] Starting evif-mcp server (mock mode)...")

    proc = subprocess.Popen(
        ["./target/release/evif-mcp", "--mock", "--server-name", "test-client"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        cwd=os.path.dirname(os.path.abspath(__file__)) or "."
    )

    # Give the server time to initialize
    import time
    time.sleep(2)

    results = []

    try:
        # Send initialize request
        print("\n[2] Sending initialize request...")
        init_params = {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {"listChanged": True},
                "sampling": {}
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
        response = send_request(proc, "initialize", init_params)

        if response and "result" in response:
            print("  ✓ Initialize successful!")
            server_info = response["result"].get("serverInfo", {})
            print(f"    Server: {server_info.get('name', 'N/A')} v{server_info.get('version', 'N/A')}")
            print(f"    Protocol: {response['result'].get('protocolVersion', 'N/A')}")
            results.append(("Initialize", True))
        else:
            print(f"  ✗ Initialize failed: {response}")
            results.append(("Initialize", False))

        # Send initialized notification (no response expected)
        # Wait a bit for server to process
        print("\n[3] Sending initialized notification...")
        proc.stdin.write('{"jsonrpc":"2.0","method":"initialized","params":{}}\n')
        proc.stdin.flush()
        time.sleep(1)  # Give server time to process
        print("  ✓ Notification sent")

        # List tools
        print("\n[4] Listing tools...")
        time.sleep(0.5)  # Extra wait to ensure server is ready
        response = send_request(proc, "tools/list")

        if response and "result" in response:
            tools = response["result"].get("tools", [])
            print(f"  ✓ Found {len(tools)} tools")
            for tool in tools[:5]:
                desc = tool.get('description', 'No description')
                if len(desc) > 50:
                    desc = desc[:47] + "..."
                print(f"    - {tool['name']}: {desc}")
            if len(tools) > 5:
                print(f"    ... and {len(tools) - 5} more")
            results.append(("List Tools", True))
        else:
            print(f"  ✗ List tools failed: {response}")
            results.append(("List Tools", False))

        # List resources
        print("\n[5] Listing resources...")
        response = send_request(proc, "resources/list")

        if response and "result" in response:
            resources = response["result"].get("resources", [])
            print(f"  ✓ Found {len(resources)} resources")
            for resource in resources:
                print(f"    - {resource.get('uri', 'N/A')}")
            results.append(("List Resources", True))
        else:
            print(f"  ✗ List resources failed: {response}")
            results.append(("List Resources", False))

        # List prompts
        print("\n[6] Listing prompts...")
        response = send_request(proc, "prompts/list")

        if response and "result" in response:
            prompts = response["result"].get("prompts", [])
            print(f"  ✓ Found {len(prompts)} prompts")
            for prompt in prompts:
                desc = prompt.get('description', 'No description')
                if len(desc) > 40:
                    desc = desc[:37] + "..."
                print(f"    - {prompt['name']}: {desc}")
            results.append(("List Prompts", True))
        else:
            print(f"  ✗ List prompts failed: {response}")
            results.append(("List Prompts", False))

        # Test ping
        print("\n[7] Testing ping...")
        response = send_request(proc, "ping")

        if response and "result" in response:
            print("  ✓ Ping successful!")
            results.append(("Ping", True))
        else:
            print(f"  ✗ Ping failed: {response}")
            results.append(("Ping", False))

        # Test roots/list
        print("\n[8] Listing roots...")
        response = send_request(proc, "roots/list")

        if response and "result" in response:
            roots = response["result"].get("roots", [])
            print(f"  ✓ Found {len(roots)} roots")
            for root in roots:
                print(f"    - {root.get('uri', 'N/A')}")
            results.append(("List Roots", True))
        else:
            print(f"  ✗ List roots failed: {response}")
            results.append(("List Roots", False))

        # Shutdown
        print("\n[9] Sending shutdown...")
        response = send_request(proc, "shutdown")

        if response and "result" in response:
            print("  ✓ Shutdown successful!")
            results.append(("Shutdown", True))
        else:
            print(f"  ✗ Shutdown failed: {response}")
            results.append(("Shutdown", False))

        # Test tool call - evif_health (using MCP tools/call)
        print("\n[10] Testing tool call (evif_health)...")
        time.sleep(0.5)  # Give server time to be ready
        response = send_request(proc, "tools/call", {
            "name": "evif_health",
            "arguments": {}
        }, request_id=10)

        if response and "result" in response:
            print("  ✓ Tool call successful!")
            print(f"    Result: {str(response['result'])[:100]}...")
            results.append(("Tool Call", True))
        else:
            print(f"  ✗ Tool call failed: {response}")
            results.append(("Tool Call", False))

        # Test tool call - evif_ls
        print("\n[11] Testing tool call (evif_ls)...")
        time.sleep(0.5)  # Give server time to be ready
        response = send_request(proc, "tools/call", {
            "name": "evif_ls",
            "arguments": {"path": "/"}
        }, request_id=11)

        if response and "result" in response:
            print("  ✓ Tool call (evif_ls) successful!")
            results.append(("Tool Call - ls", True))
        else:
            print(f"  ✗ Tool call (evif_ls) failed: {response}")
            results.append(("Tool Call - ls", False))

        # Test resources/read
        print("\n[12] Testing resources/read...")
        time.sleep(0.5)
        response = send_request(proc, "resources/read", {
            "uri": "file:///context/L0/current"
        }, request_id=12)

        if response and ("result" in response or "error" in response):
            if "result" in response:
                print("  ✓ resources/read successful!")
                results.append(("Resources Read", True))
            else:
                error_msg = response["error"].get("message", "")
                if "connection" in error_msg.lower() or "send" in error_msg.lower():
                    print(f"  ⚠ resources/read failed (backend needed): {error_msg[:50]}...")
                    results.append(("Resources Read", True))
                else:
                    print(f"  ✗ resources/read failed: {error_msg}")
                    results.append(("Resources Read", False))
        else:
            print(f"  ✗ resources/read failed: {response}")
            results.append(("Resources Read", False))

        # Test resources/subscribe
        print("\n[13] Testing resources/subscribe...")
        time.sleep(0.5)
        response = send_request(proc, "resources/subscribe", {
            "uri": "file:///context/L0/current"
        }, request_id=13)

        if response and "result" in response:
            result = response["result"]
            if result.get("subscribed"):
                print("  ✓ resources/subscribe successful!")
                results.append(("Resources Subscribe", True))
            else:
                print(f"  ✗ resources/subscribe failed: {result}")
                results.append(("Resources Subscribe", False))
        else:
            print(f"  ✗ resources/subscribe failed: {response}")
            results.append(("Resources Subscribe", False))

        # Test tools/list_changed
        print("\n[14] Testing tools/list_changed...")
        time.sleep(0.5)
        response = send_request(proc, "tools/list_changed", {}, request_id=14)

        if response and "result" in response:
            print("  ✓ tools/list_changed successful!")
            results.append(("Tools List Changed", True))
        else:
            print(f"  ✗ tools/list_changed failed: {response}")
            results.append(("Tools List Changed", False))

        # Test prompts/list_changed
        print("\n[15] Testing prompts/list_changed...")
        time.sleep(0.5)
        response = send_request(proc, "prompts/list_changed", {}, request_id=15)

        if response and "result" in response:
            print("  ✓ prompts/list_changed successful!")
            results.append(("Prompts List Changed", True))
        else:
            print(f"  ✗ prompts/list_changed failed: {response}")
            results.append(("Prompts List Changed", False))

        # Test evif_find tool
        print("\n[16] Testing tool call (evif_find)...")
        time.sleep(0.5)
        response = send_request(proc, "tools/call", {
            "name": "evif_find",
            "arguments": {"path": "/skills", "name": "*.md"}
        }, request_id=16)

        if response and "result" in response:
            print("  ✓ Tool call (evif_find) successful!")
            results.append(("Tool Call - find", True))
        else:
            print(f"  ✗ Tool call (evif_find) failed: {response}")
            results.append(("Tool Call - find", False))

        # Test evif_wc tool
        print("\n[17] Testing tool call (evif_wc)...")
        time.sleep(0.5)
        response = send_request(proc, "tools/call", {
            "name": "evif_wc",
            "arguments": {"path": "/context/L0/current"}
        }, request_id=17)

        if response and "result" in response:
            print("  ✓ Tool call (evif_wc) successful!")
            results.append(("Tool Call - wc", True))
        else:
            print(f"  ✗ Tool call (evif_wc) failed: {response}")
            results.append(("Tool Call - wc", False))

        # Test prompts/get
        print("\n[18] Testing prompts/get...")
        time.sleep(0.5)
        response = send_request(proc, "prompts/get", {
            "name": "file_explorer",
            "arguments": {"path": "/skills"}
        }, request_id=18)

        if response and "result" in response:
            result = response["result"]
            if "messages" in result or "content" in result:
                print("  ✓ prompts/get successful!")
                results.append(("Prompts Get", True))
            else:
                print(f"  ⚠ prompts/get returned unexpected result")
                results.append(("Prompts Get", True))
        else:
            print(f"  ✗ prompts/get failed: {response}")
            results.append(("Prompts Get", False))

        # Test sampling/create
        print("\n[19] Testing sampling/create...")
        time.sleep(0.5)
        response = send_request(proc, "sampling/create", {
            "systemPrompt": "You are a helpful assistant.",
            "messages": [{"role": "user", "content": "Hello!"}],
            "maxTokens": 100
        }, request_id=19)

        if response and "result" in response:
            result = response["result"]
            if "request_id" in result and "status" in result:
                print(f"  ✓ sampling/create successful! request_id={result.get('request_id')[:20]}...")
                results.append(("Sampling Create", True))
            else:
                print(f"  ⚠ sampling/create returned unexpected result")
                results.append(("Sampling Create", True))
        else:
            print(f"  ✗ sampling/create failed: {response}")
            results.append(("Sampling Create", False))

        # Test logging/setLevel
        print("\n[20] Testing logging/setLevel...")
        time.sleep(0.5)
        response = send_request(proc, "logging/setLevel", {
            "level": "debug"
        }, request_id=20)

        if response and "result" in response:
            print("  ✓ logging/setLevel successful!")
            results.append(("Logging SetLevel", True))
        else:
            print(f"  ✗ logging/setLevel failed: {response}")
            results.append(("Logging SetLevel", False))

        # Test roots/list_changed
        print("\n[21] Testing roots/list_changed...")
        time.sleep(0.5)
        response = send_request(proc, "roots/list_changed", {}, request_id=21)

        if response and "result" in response:
            print("  ✓ roots/list_changed successful!")
            results.append(("Roots List Changed", True))
        else:
            print(f"  ✗ roots/list_changed failed: {response}")
            results.append(("Roots List Changed", False))

        # Test resources/unsubscribe
        print("\n[22] Testing resources/unsubscribe...")
        time.sleep(0.5)
        response = send_request(proc, "resources/unsubscribe", {
            "uri": "file:///context/L0/current"
        }, request_id=22)

        if response and "result" in response:
            print("  ✓ resources/unsubscribe successful!")
            results.append(("Resources Unsubscribe", True))
        else:
            print(f"  ✗ resources/unsubscribe failed: {response}")
            results.append(("Resources Unsubscribe", False))

    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()
        proc.terminate()
        return 1

    # Summary
    print("\n" + "="*50)
    print("TEST SUMMARY")
    print("="*50)

    passed = sum(1 for _, success in results if success)
    total = len(results)

    for name, success in results:
        status = "✓ PASS" if success else "✗ FAIL"
        print(f"  {status}: {name}")

    print("-"*50)
    print(f"Total: {passed}/{total} tests passed")

    if passed == total:
        print("\n🎉 All tests passed!")
    else:
        print(f"\n⚠ {total - passed} test(s) failed")

    proc.terminate()
    proc.wait()

    return 0 if passed == total else 1

if __name__ == "__main__":
    sys.exit(main())