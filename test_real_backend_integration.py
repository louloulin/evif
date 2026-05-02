#!/usr/bin/env python3
"""Real Backend Integration Test - validates MCP tools against actual EVIF backend"""

import subprocess
import json
import sys
import select
import os
import time
import re

class RealBackendTest:
    def __init__(self):
        self.proc = None
        self.evif_url = os.environ.get("EVIF_URL", "http://localhost:8081")

    def start(self, use_mock=False):
        """Start the MCP server"""
        args = ["./target/release/evif-mcp"]
        if use_mock:
            args.extend(["--mock", "--server-name", "test-real"])
        else:
            args.extend(["--server-name", "real-backend"])

        self.proc = subprocess.Popen(
            args,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
            cwd="/Users/louloulin/Documents/linchong/claude/evif"
        )
        time.sleep(2)

        while True:
            ready, _, _ = select.select([self.proc.stderr], [], [], 0.5)
            if ready:
                line = self.proc.stderr.readline()
                if not line:
                    break
            else:
                break

    def send_json(self, method, params=None, req_id=1):
        """Send JSON-RPC request"""
        request = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
        }
        if params:
            request["params"] = params

        req_json = json.dumps(request) + "\n"
        self.proc.stdin.write(req_json.encode('utf-8'))
        self.proc.stdin.flush()
        return req_id

    def read_responses(self, timeout=5):
        """Read all JSON responses"""
        ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

        try:
            self.proc.stdin.close()
        except:
            pass

        time.sleep(3)

        all_data = b''
        end_time = time.time() + timeout
        while time.time() < end_time:
            ready, _, _ = select.select([self.proc.stdout], [], [], 1)
            if ready:
                chunk = self.proc.stdout.read(4096)
                if chunk:
                    all_data += chunk
                else:
                    break
            else:
                break

        responses = {}
        if all_data:
            lines = all_data.decode('utf-8', errors='replace').split('\n')
            for line in lines:
                line = ansi_escape.sub('', line.strip())
                if line.startswith('{'):
                    try:
                        obj = json.loads(line)
                        rid = obj.get("id")
                        if rid is not None:
                            responses[rid] = obj
                    except:
                        pass

        return responses

    def stop(self):
        if self.proc:
            try:
                self.proc.wait(timeout=2)
            except:
                self.proc.kill()


def test_tool_response(tool_name, args, resp):
    """Validate a tool response structure and content"""
    if resp is None:
        return False, f"{tool_name}: No response"

    if "error" in resp:
        err = resp["error"]
        # 401 errors are acceptable (auth required)
        if err.get("code") == -32001 or "401" in str(err):
            return True, f"{tool_name}: Auth required (expected)"
        return False, f"{tool_name}: Error - {err}"

    if "result" not in resp:
        return False, f"{tool_name}: No result"

    result = resp["result"]

    # Validate response has content
    if isinstance(result, dict):
        if not result:
            return False, f"{tool_name}: Empty result dict"
        # Check for common response fields
        has_content = any(k for k in result.keys() if k not in ['success', 'cached'])
        if not has_content:
            return False, f"{tool_name}: No content in result"
    elif isinstance(result, list):
        pass  # List responses are valid
    elif isinstance(result, str):
        if not result:
            return False, f"{tool_name}: Empty string result"
    elif isinstance(result, bool):
        pass  # Boolean results are valid
    else:
        return False, f"{tool_name}: Unexpected result type {type(result)}"

    return True, f"{tool_name}: OK"


def run_tests():
    print("=" * 60)
    print("REAL BACKEND INTEGRATION TEST")
    print("=" * 60)

    client = RealBackendTest()
    results = []
    errors = []

    # Start WITHOUT mock to test real backend
    print(f"\n[1] Starting MCP server (real backend at {client.evif_url})...")
    client.start(use_mock=False)

    try:
        # Initialize
        print("[2] Sending MCP requests...")

        client.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {"roots": {}, "sampling": {}},
            "clientInfo": {"name": "real-test", "version": "1.0"}
        }, req_id=1)

        client.send_json("ping", req_id=2)
        client.send_json("tools/list", req_id=3)

        # Test file operations against real backend
        file_ops = [
            ("evif_health", {}, 10),
            ("evif_ls", {"path": "/"}, 11),
            ("evif_cat", {"path": "/hello/message"}, 12),
            ("evif_stat", {"path": "/hello"}, 13),
        ]

        for tool_name, args, req_id in file_ops:
            client.send_json("tools/call", {
                "name": tool_name,
                "arguments": args
            }, req_id=req_id)

        # Test write (should get 401 without auth, but validates the path)
        client.send_json("tools/call", {
            "name": "evif_write",
            "arguments": {"path": "/test-write.txt", "content": "test"}
        }, req_id=20)

        client.send_json("shutdown", req_id=99)

        print("  ✓ Requests sent, reading responses...")

        responses = client.read_responses(timeout=5)
        print(f"  ✓ Got {len(responses)} responses")

        print("\n[3] Validating responses...")

        # Validate initialize
        resp = responses.get(1)
        if resp and "result" in resp:
            print("  ✓ Initialize OK")
            results.append("Initialize")
        else:
            print(f"  ✗ Initialize failed")
            errors.append("Initialize")

        # Validate ping
        resp = responses.get(2)
        if resp and "result" in resp:
            print("  ✓ Ping OK")
            results.append("Ping")
        else:
            print(f"  ✗ Ping failed")
            errors.append("Ping")

        # Validate tools/list
        resp = responses.get(3)
        if resp and "result" in resp:
            tools = resp["result"].get("tools", [])
            print(f"  ✓ tools/list OK ({len(tools)} tools)")
            results.append("ToolsList")
        else:
            print(f"  ✗ tools/list failed")
            errors.append("ToolsList")

        # Validate each tool call
        tool_validations = [
            ("evif_health", responses.get(10)),
            ("evif_ls", responses.get(11)),
            ("evif_cat", responses.get(12)),
            ("evif_stat", responses.get(13)),
        ]

        for tool_name, resp in tool_validations:
            success, msg = test_tool_response(tool_name, {}, resp)
            status = "✓" if success else "✗"
            print(f"  {status} {msg}")
            if success:
                results.append(tool_name)
            else:
                errors.append(f"{tool_name}: {msg}")

        # Validate write (may fail auth but should get response)
        resp = responses.get(20)
        if resp:
            if "result" in resp or ("error" in resp and resp["error"].get("code") == -32001):
                print("  ✓ evif_write OK (auth check working)")
                results.append("evif_write")
            else:
                print(f"  ✗ evif_write failed: {resp}")
                errors.append("evif_write")
        else:
            print("  ✗ evif_write: No response")
            errors.append("evif_write")

        # Validate shutdown
        resp = responses.get(99)
        if resp and "result" in resp:
            print("  ✓ Shutdown OK")
            results.append("Shutdown")
        else:
            print(f"  ✗ Shutdown failed: {resp}")
            errors.append("Shutdown")

    except Exception as e:
        print(f"\n  ✗ Exception: {e}")
        import traceback
        traceback.print_exc()
        errors.append(f"Exception: {e}")

    finally:
        client.stop()

    # Summary
    print("\n" + "=" * 60)
    print("RESULTS SUMMARY")
    print("=" * 60)

    for r in results:
        print(f"  ✓ {r}")

    for e in errors:
        print(f"  ✗ {e}")

    total = len(results) + len(errors)
    print(f"\nTotal: {len(results)}/{total} passed")

    # Return exit code based on results
    return 0 if len(errors) == 0 else 1


if __name__ == "__main__":
    exit(run_tests())
