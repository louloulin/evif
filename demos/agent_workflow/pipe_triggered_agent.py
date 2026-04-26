#!/usr/bin/env python3
"""
Pipe-Triggered Agent Demo — 展示 pipefs 的 agent 协同模式

主 agent 向 /pipes/trigger/input 写入触发信号
pipefs 自动管理状态流转: idle → running → complete
结果写入 /pipes/trigger/output

Usage:
    python pipe_triggered_agent.py
"""

import asyncio
import json
from evif import EvifClient


async def setup(client):
    """创建 trigger pipe."""
    print("[Setup] Creating trigger pipe...")
    try:
        await client.mkdir("/pipes/trigger")
    except Exception:
        pass

    # Register a simple skill
    try:
        await client.mkdir("/skills/echo")
        await client.write(
            "/skills/echo/SKILL.md",
            "# Echo Skill\n\nEchoes back the input.\n"
        )
    except Exception:
        pass

    print("[Setup] Done")


async def trigger_agent(client, message: str):
    """向 pipe input 写入触发信号."""
    print(f"\n[Trigger] Sending: {message}")
    await client.write("/pipes/trigger/input", message)

    # Read status
    status_bytes = await client.cat("/pipes/trigger/status")
    status = status_bytes.decode() if isinstance(status_bytes, bytes) else status_bytes
    print(f"  Status: {status}")


async def read_result(client):
    """读取 pipe output."""
    output = await client.cat("/pipes/trigger/output")
    if isinstance(output, bytes):
        output = output.decode("utf-8", errors="ignore")
    return output


async def main():
    print("=" * 60)
    print("EVIF Agent Workflow Demo: Pipe-Triggered Agent")
    print("=" * 60)
    print()

    client = EvifClient(base_url="http://localhost:8081")

    try:
        await client.connect()
        print("[Connected] EVIF client connected")

        # Setup
        await setup(client)

        # Show pipe structure
        print("\n[Pipe Structure]")
        entries = await client.ls("/pipes/trigger")
        for e in entries:
            print(f"  {e.name} ({'dir' if e.is_dir else 'file'})")

        # Trigger with messages
        messages = [
            "Analyze the log file for errors",
            "Summarize today's meeting notes",
            "Generate unit tests for auth module",
        ]

        for msg in messages:
            await trigger_agent(client, msg)

            # Read result
            result = await read_result(client)
            print(f"  Output: {result!r}")

        # List all pipes
        print("\n[All Pipes]")
        pipes = await client.ls("/pipes")
        for p in pipes:
            print(f"  {p.name} ({'dir' if p.is_dir else 'file'})")

        print()

    finally:
        await client.close()
        print("[Done] Connection closed")


if __name__ == "__main__":
    asyncio.run(main())
