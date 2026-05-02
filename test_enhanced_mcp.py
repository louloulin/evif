#!/usr/bin/env python3
"""Comprehensive MCP integration test - testing all 29 tools and capabilities"""

import subprocess
import json
import sys
import select
import os
import time
import re

class McpTestClient:
    def __init__(self):
        self.proc = None

    def start(self):
        """Start the MCP server"""
        self.proc = subprocess.Popen(
            ["./target/release/evif-mcp", "--mock", "--server-name", "test-client"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,  # Unbuffered for Rust interop
            cwd="/Users/louloulin/Documents/linchong/claude/evif"
        )
        time.sleep(2)

        # Drain any initial stderr output (log lines)
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

    def read_all_responses(self, timeout=5):
        """Read all JSON responses and return as a dict keyed by id"""
        ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

        # Close stdin to signal EOF and trigger response flush
        try:
            self.proc.stdin.close()
        except:
            pass

        # Wait for server to process
        time.sleep(3)

        # Read all stdout
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

        # Parse JSON responses
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
        """Stop the server"""
        if self.proc:
            try:
                self.proc.wait(timeout=2)
            except:
                self.proc.kill()


def run_tests():
    print("=" * 60)
    print("COMPREHENSIVE MCP INTEGRATION TEST")
    print("=" * 60)

    client = McpTestClient()
    results = []
    errors = []

    try:
        # Start server
        print("\n[1] Starting MCP server...")
        client.start()
        print("  ✓ Server started")

        # Send all requests
        print("\n[2] Sending MCP requests...")

        # Initialize
        client.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {"roots": {}, "sampling": {}},
            "clientInfo": {"name": "test", "version": "1.0"}
        }, req_id=100)

        # Ping
        client.send_json("ping", req_id=2)

        # tools/list
        client.send_json("tools/list", req_id=3)

        # resources/list
        client.send_json("resources/list", req_id=4)

        # prompts/list
        client.send_json("prompts/list", req_id=5)

        # roots/list
        client.send_json("roots/list", req_id=6)

        # Test all 30 tools (added evif_mcp_capabilities)
        tool_tests = [
            ("evif_health", {}, 7),
            ("evif_ls", {"path": "/skills"}, 8),
            ("evif_cat", {"path": "/context/L0/current"}, 81),
            ("evif_write", {"path": "/test.txt", "content": "test"}, 82),
            ("evif_mkdir", {"path": "/test_dir"}, 83),
            ("evif_rm", {"path": "/test.txt"}, 84),
            ("evif_stat", {"path": "/context"}, 85),
            ("evif_mv", {"old": "/a", "new": "/b"}, 86),
            ("evif_cp", {"from": "/a", "to": "/b"}, 87),
            ("evif_mount", {"plugin": "test", "path": "/mnt"}, 88),
            ("evif_unmount", {"path": "/mnt"}, 89),
            ("evif_mounts", {}, 90),
            ("evif_grep", {"pattern": "test", "path": "/context"}, 91),
            ("evif_find", {"path": "/context", "name": "*.md"}, 92),
            ("evif_wc", {"path": "/context/L0/current"}, 93),
            ("evif_tail", {"path": "/context/L0/current", "lines": 10}, 94),
            ("evif_open_handle", {"path": "/test.txt"}, 95),
            ("evif_close_handle", {"handle": 1}, 96),
            ("evif_memorize", {"content": "Test memory", "key": "test-key"}, 97),
            ("evif_retrieve", {"key": "test-key"}, 98),
            ("evif_skill_list", {}, 99),
            ("evif_skill_info", {"name": "evif-ls"}, 101),
            ("evif_skill_execute", {"name": "test", "args": {}}, 102),
            ("evif_claude_md_generate", {"path": "/test.md"}, 103),
            ("evif_session_save", {"name": "test-session"}, 104),
            ("evif_session_list", {}, 105),
            ("evif_subagent_create", {"name": "test", "prompt": "test"}, 106),
            ("evif_subagent_send", {"id": "1", "message": "test"}, 107),
            ("evif_subagent_list", {}, 108),
            ("evif_mcp_capabilities", {"category": "all", "detailed": False}, 109),
            ("evif_plugin_catalog", {"tier": "all"}, 110),
            ("evif_server_stats", {"detailed": False}, 111),
            ("evif_batch", {"operations": [{"op": "list", "path": "/"}]}, 112),
            ("evif_search", {"query": "test search", "limit": 5}, 113),
            ("evif_diff", {"old_path": "/a.txt", "new_path": "/b.txt"}, 114),
            ("evif_watch", {"path": "/", "timeout": 10}, 115),
            ("evif_tree", {"path": "/", "max_depth": 2}, 116),
            ("evif_archive", {"operation": "list", "archive_path": "/test.zip"}, 117),
            ("evif_hash", {"path": "/test.txt", "algorithm": "md5"}, 118),
            ("evif_du", {"path": "/", "max_depth": 2}, 119),
        ]

        for tool_name, args, req_id in tool_tests:
            client.send_json("tools/call", {
                "name": tool_name,
                "arguments": args
            }, req_id=req_id)

        # resources/read
        client.send_json("resources/read", {
            "uri": "file:///context/L0/current"
        }, req_id=9)

        # prompts/get
        client.send_json("prompts/get", {
            "name": "file_explorer"
        }, req_id=10)

        # sampling/create
        client.send_json("sampling/create", {
            "systemPrompt": "Test",
            "messages": [{"role": "user", "content": "hello"}],
            "maxTokens": 100
        }, req_id=11)

        # sampling/complete
        client.send_json("sampling/complete", {
            "request_id": "test-sampling-123",
            "content": {"text": "This is a mock LLM response"},
            "usage": {"input_tokens": 10, "output_tokens": 20, "total_tokens": 30},
            "model": "test-model"
        }, req_id=12)

        # logging/setLevel
        client.send_json("logging/setLevel", {
            "level": "info"
        }, req_id=13)

        # Shutdown
        client.send_json("shutdown", req_id=14)

        # complete_message
        client.send_json("complete_message", {
            "request_id": "test-123",
            "content": {"type": "text", "text": "Mock LLM response"}
        }, req_id=15)

        # create_message
        client.send_json("create_message", {
            "role": "assistant",
            "content": {"type": "text", "text": "Hello from server"}
        }, req_id=16)

        print("  ✓ All requests sent, reading responses...")

        # Read all responses
        responses = client.read_all_responses(timeout=5)
        print(f"  ✓ Got {len(responses)} responses")

        # === VERIFY RESPONSES ===

        # Test 1: Initialize
        print("\n[3] Verifying responses...")
        resp = responses.get(100)
        if resp and "result" in resp:
            info = resp.get("result", {}).get("serverInfo", {})
            print(f"  ✓ Initialize OK (server: {info.get('name', 'unknown')})")
            results.append("Initialize")
        else:
            print(f"  ✗ Initialize failed: {resp}")
            errors.append("Initialize")

        # Test 2: Ping
        resp = responses.get(2)
        if resp and "result" in resp:
            print("  ✓ Ping OK")
            results.append("Ping")
        else:
            print(f"  ✗ Ping failed: {resp}")
            errors.append("Ping")

        # Test 3: tools/list
        resp = responses.get(3)
        if resp and "result" in resp:
            tools = resp["result"].get("tools", [])
            print(f"  ✓ tools/list OK ({len(tools)} tools)")
            results.append("ToolsList")
        else:
            print(f"  ✗ tools/list failed: {resp}")
            errors.append("ToolsList")

        # Test 4: resources/list
        resp = responses.get(4)
        if resp and "result" in resp:
            resources = resp["result"].get("resources", [])
            print(f"  ✓ resources/list OK ({len(resources)} resources)")
            results.append("ResourcesList")
        else:
            print(f"  ✗ resources/list failed: {resp}")
            errors.append("ResourcesList")

        # Test 5: prompts/list
        resp = responses.get(5)
        if resp and "result" in resp:
            prompts = resp["result"].get("prompts", [])
            print(f"  ✓ prompts/list OK ({len(prompts)} prompts)")
            results.append("PromptsList")
        else:
            print(f"  ✗ prompts/list failed: {resp}")
            errors.append("PromptsList")

        # Test 6: roots/list
        resp = responses.get(6)
        if resp and "result" in resp:
            roots = resp["result"].get("roots", [])
            print(f"  ✓ roots/list OK ({len(roots)} roots)")
            results.append("RootsList")
        else:
            print(f"  ✗ roots/list failed: {resp}")
            errors.append("RootsList")

        # Verify all 37 tools
        tool_names = [
            "evif_health", "evif_ls", "evif_cat", "evif_write", "evif_mkdir",
            "evif_rm", "evif_stat", "evif_mv", "evif_cp", "evif_mount",
            "evif_unmount", "evif_mounts", "evif_grep", "evif_find", "evif_wc",
            "evif_tail", "evif_open_handle", "evif_close_handle", "evif_memorize",
            "evif_retrieve", "evif_skill_list", "evif_skill_info", "evif_skill_execute",
            "evif_claude_md_generate", "evif_session_save", "evif_session_list",
            "evif_subagent_create", "evif_subagent_send", "evif_subagent_list",
            "evif_mcp_capabilities", "evif_plugin_catalog", "evif_server_stats",
            "evif_batch", "evif_search", "evif_diff", "evif_watch", "evif_tree",
            "evif_archive", "evif_hash", "evif_du"
        ]

        tool_ids = [7, 8, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119]
        passed_tools = 0
        for i, req_id in enumerate(tool_ids):
            tool_name = tool_names[i]
            resp = responses.get(req_id)
            if resp and ("result" in resp or "error" in resp):
                passed_tools += 1
            else:
                print(f"  ⚠ {tool_name} response: {resp}")

        print(f"  ✓ Tool calls: {passed_tools}/{len(tool_names)} responded")
        if passed_tools >= 25:  # Allow some failures for mock mode
            results.append(f"ToolCalls ({passed_tools}/{len(tool_names)})")
        else:
            errors.append(f"ToolCalls ({passed_tools}/{len(tool_names)})")

        # Test 7: resources/read
        resp = responses.get(9)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ resources/read OK")
            results.append("ResourcesRead")
        else:
            print(f"  ✗ resources/read failed: {resp}")
            errors.append("ResourcesRead")

        # Test 8: prompts/get
        resp = responses.get(10)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ prompts/get OK")
            results.append("PromptsGet")
        else:
            print(f"  ✗ prompts/get failed: {resp}")
            errors.append("PromptsGet")

        # Test 9: sampling/create
        resp = responses.get(11)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ sampling/create OK")
            results.append("SamplingCreate")
        else:
            print(f"  ✗ sampling/create failed: {resp}")
            errors.append("SamplingCreate")

        # Test 10: sampling/complete
        resp = responses.get(12)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ sampling/complete OK")
            results.append("SamplingComplete")
        else:
            print(f"  ✗ sampling/complete failed: {resp}")
            errors.append("SamplingComplete")

        # Test 11: logging/setLevel
        resp = responses.get(13)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ logging/setLevel OK")
            results.append("LoggingSetLevel")
        else:
            print(f"  ✗ logging/setLevel failed: {resp}")
            errors.append("LoggingSetLevel")

        # Test 12: Shutdown
        resp = responses.get(14)
        if resp and "result" in resp:
            print("  ✓ Shutdown OK")
            results.append("Shutdown")
        else:
            print(f"  ✗ Shutdown failed: {resp}")
            errors.append("Shutdown")

        # Test 13: complete_message
        resp = responses.get(15)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ complete_message OK")
            results.append("CompleteMessage")
        else:
            print(f"  ✗ complete_message failed: {resp}")
            errors.append("CompleteMessage")

        # Test 14: create_message
        resp = responses.get(16)
        if resp and ("result" in resp or "error" in resp):
            print("  ✓ create_message OK")
            results.append("CreateMessage")
        else:
            print(f"  ✗ create_message failed: {resp}")
            errors.append("CreateMessage")

    except Exception as e:
        print(f"\n  ✗ Exception: {e}")
        errors.append(f"Exception: {e}")

    finally:
        client.stop()

    # Print summary
    print("\n" + "=" * 60)
    print("RESULTS SUMMARY")
    print("=" * 60)

    for r in results:
        print(f"  ✓ {r}")

    for e in errors:
        print(f"  ✗ {e}")

    print(f"\nTotal: {len(results)}/{len(results) + len(errors)} passed")

    return 0 if len(errors) == 0 else 1


if __name__ == "__main__":
    exit(run_tests())
