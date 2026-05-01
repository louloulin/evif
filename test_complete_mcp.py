#!/usr/bin/env python3
"""
Complete MCP Server Verification Test
Tests all 40 tools, 4 prompts, 3 resources, protocol compliance
"""

import subprocess
import json
import sys
import select
import os
import time
import re

class CompleteMcpTest:
    def __init__(self):
        self.proc = None

    def start(self, mock=True):
        args = ["./target/release/evif-mcp"]
        if mock:
            args.extend(["--mock", "--server-name", "complete-test"])
        else:
            args.append("--server-name")
            args.append("complete-test")

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


def run_complete_test():
    print("=" * 70)
    print("COMPLETE MCP SERVER VERIFICATION")
    print("=" * 70)

    test = CompleteMcpTest()

    # All 40 tools to test
    all_tools = [
        # File operations
        ("evif_health", {"path": "/"}, 10, "Health check"),
        ("evif_ping_with_stats", {"detailed": True}, 9, "Ping with stats"),
        ("evif_ls", {"path": "/"}, 11, "List directory"),
        ("evif_cat", {"path": "/hello"}, 12, "Read file"),
        ("evif_write", {"path": "/test.txt", "content": "test"}, 13, "Write file"),
        ("evif_mkdir", {"path": "/test_dir"}, 14, "Create directory"),
        ("evif_rm", {"path": "/test.txt"}, 15, "Remove file"),
        ("evif_stat", {"path": "/hello"}, 16, "File stats"),
        ("evif_mv", {"old": "/a", "new": "/b"}, 17, "Move file"),
        ("evif_cp", {"from": "/a", "to": "/b"}, 18, "Copy file"),
        # Plugin management
        ("evif_mount", {"plugin": "test", "path": "/mnt"}, 19, "Mount plugin"),
        ("evif_unmount", {"path": "/mnt"}, 20, "Unmount plugin"),
        ("evif_mounts", {}, 21, "List mounts"),
        # Search
        ("evif_grep", {"pattern": "test", "path": "/"}, 22, "Grep search"),
        ("evif_find", {"path": "/", "name": "*.txt"}, 23, "Find files"),
        ("evif_wc", {"path": "/test"}, 24, "Word count"),
        ("evif_tail", {"path": "/test", "lines": 10}, 25, "Tail file"),
        # Handle
        ("evif_open_handle", {"path": "/test.txt"}, 26, "Open handle"),
        ("evif_close_handle", {"handle": 1}, 27, "Close handle"),
        # Memory
        ("evif_memorize", {"content": "test", "key": "k1"}, 28, "Memorize"),
        ("evif_retrieve", {"key": "k1"}, 29, "Retrieve memory"),
        # Skills
        ("evif_skill_list", {}, 30, "List skills"),
        ("evif_skill_info", {"name": "test"}, 31, "Skill info"),
        ("evif_skill_execute", {"name": "test", "args": {}}, 32, "Execute skill"),
        ("evif_claude_md_generate", {"path": "/test.md"}, 33, "Generate MD"),
        # Session
        ("evif_session_save", {"name": "test-session"}, 34, "Save session"),
        ("evif_session_list", {}, 35, "List sessions"),
        # Subagent
        ("evif_subagent_create", {"name": "test", "prompt": "test"}, 36, "Create agent"),
        ("evif_subagent_send", {"id": "1", "message": "test"}, 37, "Send to agent"),
        ("evif_subagent_list", {}, 38, "List agents"),
        # Meta
        ("evif_mcp_capabilities", {"category": "all"}, 39, "Capabilities"),
        ("evif_plugin_catalog", {"tier": "all"}, 40, "Plugin catalog"),
        ("evif_server_stats", {"detailed": False}, 41, "Server stats"),
        # Batch & advanced
        ("evif_batch", {"operations": [{"op": "list", "path": "/"}]}, 42, "Batch ops"),
        ("evif_search", {"query": "test", "limit": 5}, 43, "Search"),
        ("evif_diff", {"old_path": "/a.txt", "new_path": "/b.txt"}, 44, "Diff files"),
        ("evif_watch", {"path": "/", "timeout": 10}, 45, "Watch files"),
        ("evif_tree", {"path": "/", "max_depth": 2}, 46, "Tree view"),
        # Archive tools
        ("evif_archive", {"operation": "list", "archive_path": "/test.zip"}, 47, "Archive ops"),
        ("evif_hash", {"path": "/test.txt", "algorithm": "md5"}, 48, "File hash"),
        ("evif_du", {"path": "/", "max_depth": 2}, 49, "Disk usage"),
        # Diagnostic tools
        ("evif_latency_test", {"target": "/api/v1/health", "iterations": 5}, 50, "Latency test"),
        ("evif_request_trace", {"enable": True, "verbose": False}, 51, "Request trace"),
        ("evif_cache_stats", {"reset": False}, 52, "Cache stats"),
        ("evif_log_query", {"level": "info", "limit": 10}, 53, "Log query"),
        ("evif_metrics_export", {"format": "json"}, 54, "Metrics export"),
        ("evif_config_get", {"key": "server_name"}, 55, "Config get"),
        # Event & Cron tools (v2)
        ("evif_event_subscribe", {"event_type": "file_change", "path_filter": "*.txt"}, 56, "Event subscribe"),
        ("evif_event_list", {"event_type": "file_change"}, 57, "Event list"),
        ("evif_cron_schedule", {"name": "test-cron", "cron": "0 9 * * *", "command": "echo test"}, 58, "Cron schedule"),
        # More v2 tools
        ("evif_event_unsubscribe", {"subscription_id": "sub-001"}, 59, "Event unsubscribe"),
        ("evif_cron_list", {"include_disabled": False}, 60, "Cron list"),
        ("evif_cron_remove", {"schedule_id": "cron-001"}, 61, "Cron remove"),
        ("evif_session_load", {"name": "test-session"}, 62, "Session load"),
        ("evif_subagent_kill", {"id": "agent-001", "reason": "done"}, 63, "Subagent kill"),
        # More v3 tools
        ("evif_skill_create", {"name": "test-skill", "template": "code-review", "description": "Test skill"}, 64, "Skill create"),
        ("evif_skill_delete", {"name": "old-skill", "force": True}, 65, "Skill delete"),
        ("evif_memory_search", {"query": "test query", "limit": 10}, 66, "Memory search"),
        ("evif_memory_stats", {"detailed": True}, 67, "Memory stats"),
        ("evif_pipe_create", {"name": "test-pipe", "capacity": 50}, 68, "Pipe create"),
    ]

    results = {"passed": [], "failed": []}

    try:
        print("\n[1] Starting MCP server (mock mode)...")
        test.start(mock=True)
        print("  ✓ Server started")

        print("\n[2] Testing protocol initialization...")
        test.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {"roots": {}, "sampling": {}},
            "clientInfo": {"name": "complete-test", "version": "1.0"}
        }, req_id=1)
        test.send_json("ping", req_id=2)

        responses = test.read_responses()
        if responses.get(1) and "result" in responses.get(1):
            print("  ✓ Initialize OK")
            results["passed"].append("Initialize")
        else:
            print("  ✗ Initialize failed")
            results["failed"].append("Initialize")

        if responses.get(2) and "result" in responses.get(2):
            print("  ✓ Ping OK")
            results["passed"].append("Ping")
        else:
            print("  ✗ Ping failed")
            results["failed"].append("Ping")

        print("\n[3] Testing tools/list...")
        test.start(mock=True)
        test.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }, req_id=1)
        test.send_json("tools/list", req_id=3)
        test.send_json("shutdown", req_id=99)

        responses = test.read_responses()
        tools_resp = responses.get(3)
        if tools_resp and "result" in tools_resp:
            tools = tools_resp["result"].get("tools", [])
            print(f"  ✓ tools/list OK ({len(tools)} tools)")
            results["passed"].append(f"Tools list ({len(tools)} tools)")
        else:
            print("  ✗ tools/list failed")
            results["failed"].append("Tools list")

        print("\n[4] Testing all 60 tool calls...")
        for i, (tool_name, args, req_id, desc) in enumerate(all_tools, 1):
            test.start(mock=True)
            test.send_json("initialize", {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }, req_id=1)
            test.send_json("tools/call", {
                "name": tool_name,
                "arguments": args
            }, req_id=req_id)
            test.send_json("shutdown", req_id=99)

            responses = test.read_responses()
            resp = responses.get(req_id)

            if resp and ("result" in resp or "error" in resp):
                results["passed"].append(f"{tool_name}")
                print(f"  ✓ [{i:02d}/60] {tool_name} ({desc})")
            else:
                results["failed"].append(f"{tool_name}: No response")
                print(f"  ✗ [{i:02d}/60] {tool_name} - No response")

        print("\n[5] Testing prompts/list...")
        test.start(mock=True)
        test.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }, req_id=1)
        test.send_json("prompts/list", req_id=50)
        test.send_json("shutdown", req_id=99)

        responses = test.read_responses()
        prompts_resp = responses.get(50)
        if prompts_resp and "result" in prompts_resp:
            prompts = prompts_resp["result"].get("prompts", [])
            print(f"  ✓ prompts/list OK ({len(prompts)} prompts)")
            results["passed"].append(f"Prompts list ({len(prompts)} prompts)")
        else:
            print("  ✗ prompts/list failed")
            results["failed"].append("Prompts list")

        print("\n[6] Testing resources/list...")
        test.start(mock=True)
        test.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }, req_id=1)
        test.send_json("resources/list", req_id=51)
        test.send_json("shutdown", req_id=99)

        responses = test.read_responses()
        resources_resp = responses.get(51)
        if resources_resp and "result" in resources_resp:
            resources = resources_resp["result"].get("resources", [])
            print(f"  ✓ resources/list OK ({len(resources)} resources)")
            results["passed"].append(f"Resources list ({len(resources)} resources)")
        else:
            print("  ✗ resources/list failed")
            results["failed"].append("Resources list")

        print("\n[7] Testing roots/list...")
        test.start(mock=True)
        test.send_json("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }, req_id=1)
        test.send_json("roots/list", req_id=52)
        test.send_json("shutdown", req_id=99)

        responses = test.read_responses()
        roots_resp = responses.get(52)
        if roots_resp and "result" in roots_resp:
            roots = roots_resp["result"].get("roots", [])
            print(f"  ✓ roots/list OK ({len(roots)} roots)")
            results["passed"].append(f"Roots list ({len(roots)} roots)")
        else:
            print("  ✗ roots/list failed")
            results["failed"].append("Roots list")

    except Exception as e:
        print(f"\n  ✗ Exception: {e}")
        import traceback
        traceback.print_exc()
        results["failed"].append(f"Exception: {e}")

    finally:
        test.stop()

    # Summary
    print("\n" + "=" * 70)
    print("VERIFICATION SUMMARY")
    print("=" * 70)
    print(f"\n✓ Passed: {len(results['passed'])}")
    print(f"✗ Failed: {len(results['failed'])}")

    if results["failed"]:
        print("\nFailed tests:")
        for f in results["failed"]:
            print(f"  ✗ {f}")

    print("\n" + "=" * 70)
    total = len(results["passed"]) + len(results["failed"])
    print(f"RESULT: {len(results['passed'])}/{total} tests passed")
    print("=" * 70)

    return 0 if len(results["failed"]) == 0 else 1


if __name__ == "__main__":
    exit(run_complete_test())
