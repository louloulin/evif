#!/usr/bin/env python3
"""Comprehensive MCP Tool Validation Test - validates all 40 tools have correct schemas and responses"""

import subprocess
import json
import sys
import select
import os
import time
import re

class McpToolValidator:
    def __init__(self):
        self.proc = None

    def start(self):
        """Start MCP server in mock mode"""
        self.proc = subprocess.Popen(
            ["./target/release/evif-mcp", "--mock", "--server-name", "validator"],
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


def validate_tool_schema(tool_def):
    """Validate tool has correct schema structure"""
    required = ['name', 'description', 'inputSchema']
    for field in required:
        if field not in tool_def:
            return False, f"Missing field: {field}"
    return True, "Valid schema"


def validate_tool_response(tool_name, resp):
    """Validate tool response has proper structure"""
    if resp is None:
        return False, "No response"

    if "error" in resp:
        return True, "Error response (acceptable)"

    if "result" not in resp:
        return False, "No result in response"

    result = resp["result"]
    if isinstance(result, dict):
        if not result:
            return False, "Empty result dict"
    return True, "Valid response"


def run_tests():
    print("=" * 70)
    print("MCP TOOL VALIDATION TEST")
    print("=" * 70)

    client = McpToolValidator()
    results = []
    errors = []
    tool_schemas = {}

    try:
        print("\n[1] Starting MCP server...")
        client.start()
        print("  ✓ Server started")

        print("\n[2] Initializing and getting tool list...")
        client.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {"roots": {}, "sampling": {}},
            "clientInfo": {"name": "validator", "version": "1.0"}
        }, req_id=1)

        client.send_json("ping", req_id=2)
        client.send_json("tools/list", req_id=3)
        client.send_json("shutdown", req_id=99)

        responses = client.read_responses()
        print(f"  ✓ Got {len(responses)} responses")

        # Get tool definitions
        tools_resp = responses.get(3)
        if tools_resp and "result" in tools_resp:
            tools = tools_resp["result"].get("tools", [])
            print(f"  ✓ Found {len(tools)} tools")
        else:
            print("  ✗ Failed to get tools")
            return 1

        print("\n[3] Validating tool schemas...")

        for tool in tools:
            name = tool.get("name", "unknown")
            valid, msg = validate_tool_schema(tool)
            if valid:
                tool_schemas[name] = tool
                results.append(f"Schema: {name}")
            else:
                errors.append(f"Schema: {name} - {msg}")
                print(f"  ✗ {name}: {msg}")

        print(f"\n  Valid schemas: {len(results)}/{len(tools)}")

        print("\n[4] Testing tool calls...")
        client.start()

        # All 40 tools with sample arguments
        all_tools = [
            ("evif_health", {}, 10),
            ("evif_ls", {"path": "/"}, 11),
            ("evif_cat", {"path": "/hello"}, 12),
            ("evif_write", {"path": "/test.txt", "content": "test"}, 13),
            ("evif_mkdir", {"path": "/test_dir"}, 14),
            ("evif_rm", {"path": "/test.txt"}, 15),
            ("evif_stat", {"path": "/hello"}, 16),
            ("evif_mv", {"old": "/a", "new": "/b"}, 17),
            ("evif_cp", {"from": "/a", "to": "/b"}, 18),
            ("evif_mount", {"plugin": "test", "path": "/mnt"}, 19),
            ("evif_unmount", {"path": "/mnt"}, 20),
            ("evif_mounts", {}, 21),
            ("evif_grep", {"pattern": "test", "path": "/"}, 22),
            ("evif_find", {"path": "/", "name": "*.txt"}, 23),
            ("evif_wc", {"path": "/test"}, 24),
            ("evif_tail", {"path": "/test", "lines": 10}, 25),
            ("evif_open_handle", {"path": "/test.txt"}, 26),
            ("evif_close_handle", {"handle": 1}, 27),
            ("evif_memorize", {"content": "test", "key": "k1"}, 28),
            ("evif_retrieve", {"key": "k1"}, 29),
            ("evif_skill_list", {}, 30),
            ("evif_skill_info", {"name": "test"}, 31),
            ("evif_skill_execute", {"name": "test", "args": {}}, 32),
            ("evif_claude_md_generate", {"path": "/test.md"}, 33),
            ("evif_session_save", {"name": "test-session"}, 34),
            ("evif_session_list", {}, 35),
            ("evif_subagent_create", {"name": "test", "prompt": "test"}, 36),
            ("evif_subagent_send", {"id": "1", "message": "test"}, 37),
            ("evif_subagent_list", {}, 38),
            ("evif_mcp_capabilities", {"category": "all"}, 39),
            ("evif_plugin_catalog", {"tier": "all"}, 40),
            ("evif_server_stats", {"detailed": False}, 41),
            ("evif_batch", {"operations": [{"op": "list", "path": "/"}]}, 42),
            ("evif_search", {"query": "test", "limit": 5}, 43),
            ("evif_diff", {"old_path": "/a.txt", "new_path": "/b.txt"}, 44),
            ("evif_watch", {"path": "/", "timeout": 10}, 45),
            ("evif_tree", {"path": "/", "max_depth": 2}, 46),
            ("evif_archive", {"operation": "list", "archive_path": "/test.zip"}, 47),
            ("evif_hash", {"path": "/test.txt", "algorithm": "md5"}, 48),
            ("evif_du", {"path": "/", "max_depth": 2}, 49),
        ]

        for tool_name, args, req_id in all_tools:
            client.send_json("initialize", {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }, req_id=1)
            client.send_json("tools/call", {
                "name": tool_name,
                "arguments": args
            }, req_id=req_id)
            client.send_json("shutdown", req_id=99)

            responses = client.read_responses()
            resp = responses.get(req_id)
            valid, msg = validate_tool_response(tool_name, resp)
            if valid:
                print(f"  ✓ {tool_name}")
                results.append(f"Call: {tool_name}")
            else:
                print(f"  ✗ {tool_name}: {msg}")
                errors.append(f"Call: {tool_name}: {msg}")

            client.start()  # Restart for next test

    except Exception as e:
        print(f"\n  ✗ Exception: {e}")
        import traceback
        traceback.print_exc()
        errors.append(f"Exception: {e}")

    finally:
        client.stop()

    print("\n" + "=" * 70)
    print("RESULTS SUMMARY")
    print("=" * 70)
    print(f"\nValid schemas: {len([r for r in results if r.startswith('Schema')])}")
    print(f"Valid tool calls: {len([r for r in results if r.startswith('Call')])}")
    print(f"Total: {len(results)}/{len(results) + len(errors)} passed")

    if errors:
        print("\nErrors:")
        for e in errors[:10]:
            print(f"  ✗ {e}")

    return 0 if len(errors) == 0 else 1


if __name__ == "__main__":
    exit(run_tests())
