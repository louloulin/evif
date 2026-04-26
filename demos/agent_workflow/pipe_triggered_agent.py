#!/usr/bin/env python3
"""
Pipe-Triggered Agent Demo — 展示 pipefs + skillfs 的 agent 协同模式

主 agent 向 /trigger/input 写入触发信号
Worker agent 监听 /trigger/status，当状态变为 "triggered" 时执行技能
执行结果写入 /trigger/output
状态流转: idle → triggered → running → complete

Usage:
    python pipe_triggered_agent.py
"""

import asyncio
import json
from evif import EvifClient

PIPE_NAME = "trigger"
SKILL_NAME = "echo"


async def setup_pipe(client):
    """Setup pipe and skill for the demo."""
    print("[Setup] Creating pipe and skill...")

    # Create pipe directories
    for subpath in ["input", "output", "status", "assignee", "timeout"]:
        try:
            await client.mkdir(f"/pipes/{PIPE_NAME}/{subpath}")
        except Exception:
            pass

    # Initialize pipe state
    await client.write(f"/pipes/{PIPE_NAME}/status", "idle")
    await client.write(f"/pipes/{PIPE_NAME}/assignee", "")
    await client.write(f"/pipes/{PIPE_NAME}/timeout", "60")

    # Create skill
    skill_content = """# Echo Skill

Echoes back the input with a timestamp.

## Triggers
- echo
- repeat
- test

## Execution
Write input to /skills/echo/input, read result from /skills/echo/output.

## Example
Input: "hello"
Output: "[echo] 2026-04-26 hello"
"""
    try:
        await client.mkdir(f"/skills/{SKILL_NAME}")
        await client.write(f"/skills/{SKILL_NAME}/SKILL.md", skill_content)
    except Exception:
        pass

    print("[Setup] Pipe and skill created")


async def trigger_agent(client, input_data):
    """Trigger the agent by writing to pipe input."""
    print(f"[Trigger] Sending input: {input_data}")

    # Set status to triggered
    await client.write(f"/pipes/{PIPE_NAME}/status", "triggered")
    await client.write(f"/pipes/{PIPE_NAME}/assignee", "agent-1")

    # Write input data
    if isinstance(input_data, str):
        await client.write(f"/pipes/{PIPE_NAME}/input", input_data)
    else:
        await client.write(f"/pipes/{PIPE_NAME}/input", json.dumps(input_data))

    print("[Trigger] Agent triggered")


async def wait_for_status(client, target_status, timeout=10):
    """Poll until pipe reaches target status."""
    for _ in range(timeout):
        status_data = await client.cat(f"/pipes/{PIPE_NAME}/status")
        if isinstance(status_data, bytes):
            status_data = status_data.decode("utf-8", errors="ignore")

        current = status_data.strip()
        if current == target_status:
            return True
        if current in ("complete", "error"):
            return True  # Terminal state

        await asyncio.sleep(0.5)

    return False


async def execute_skill(client, input_data):
    """Execute the skill and write result."""
    print("[Execute] Running skill...")

    # Update status to running
    await client.write(f"/pipes/{PIPE_NAME}/status", "running")

    # Simulate skill execution
    await asyncio.sleep(0.2)

    # Get current input
    input_content = await client.cat(f"/pipes/{PIPE_NAME}/input")
    if isinstance(input_content, bytes):
        input_content = input_content.decode("utf-8", errors="ignore")

    # Create result
    import time
    result = {
        "skill": SKILL_NAME,
        "input": input_content,
        "output": f"[echo] Processed at {time.strftime('%Y-%m-%d %H:%M:%S')}: {input_content}",
        "status": "success"
    }

    # Write output
    await client.write(
        f"/pipes/{PIPE_NAME}/output",
        json.dumps(result, ensure_ascii=False)
    )

    # Mark complete
    await client.write(f"/pipes/{PIPE_NAME}/status", "complete")

    print("[Execute] Skill completed")
    return result


async def worker_loop(client):
    """Worker loop that monitors and processes pipe triggers."""
    print("[Worker] Starting worker loop...")

    while True:
        try:
            # Check status
            status_data = await client.cat(f"/pipes/{PIPE_NAME}/status")
            if isinstance(status_data, bytes):
                status_data = status_data.decode("utf-8", errors="ignore")

            status = status_data.strip()

            if status == "triggered":
                print("[Worker] Detected trigger!")
                result = await execute_skill(client, None)
                print(f"[Worker] Result: {result['output']}")

            await asyncio.sleep(0.2)

        except Exception as e:
            print(f"[Worker] Error: {e}")
            await asyncio.sleep(1)


async def main():
    print("=" * 60)
    print("EVIF Agent Workflow Demo: Pipe-Triggered Agent")
    print("=" * 60)
    print()

    # Create client
    client = EvifClient(
        base_url="http://localhost:8081",
        api_key="write-key",
    )

    try:
        await client.connect()
        print("[Connected] EVIF client connected\n")

        # Setup
        await setup_pipe(client)
        print()

        # Run demo: trigger -> execute -> result
        test_inputs = ["hello world", "test data", "final input"]

        for i, input_data in enumerate(test_inputs):
            print(f"[Demo {i+1}] Input: '{input_data}'")

            # Trigger
            await trigger_agent(client, input_data)

            # Execute (simulate worker processing)
            result = await execute_skill(client, input_data)

            # Read output
            output_data = await client.cat(f"/pipes/{PIPE_NAME}/output")
            if isinstance(output_data, bytes):
                output_data = output_data.decode("utf-8", errors="ignore")

            try:
                output_json = json.loads(output_data)
                print(f"  Output: {output_json.get('output', output_data)}")
            except Exception:
                print(f"  Output: {output_data}")

            # Reset for next iteration
            await client.write(f"/pipes/{PIPE_NAME}/status", "idle")
            print()

        print("=" * 60)
        print("Demo completed!")
        print("=" * 60)

        # Cleanup
        print("\n[Cleanup] Resetting pipe...")
        await client.write(f"/pipes/{PIPE_NAME}/status", "idle")
        await client.write(f"/pipes/{PIPE_NAME}/input", "")
        await client.write(f"/pipes/{PIPE_NAME}/output", "")

    finally:
        await client.close()
        print("[Done] Connection closed")


if __name__ == "__main__":
    asyncio.run(main())