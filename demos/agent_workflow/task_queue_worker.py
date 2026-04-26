#!/usr/bin/env python3
"""
Task Queue Worker Demo — 展示 queuefs + memory 的 agent 协同模式

主 agent 向 /tasks/enqueue 提交任务
Worker agent 从 /tasks/dequeue 消费任务
执行结果写入 /results/{task_id}

Usage:
    python task_queue_worker.py
"""

import asyncio
import json
import uuid
from evif import EvifClient

WORKER_ID = "worker-1"
TASK_QUEUE = "/tasks"
RESULT_BASE = "/results"


async def enqueue_task(client, task_type, payload):
    """Enqueue a task to the task queue."""
    task = {
        "id": str(uuid.uuid4()),
        "type": task_type,
        "payload": payload,
        "enqueued_at": asyncio.get_event_loop().time(),
    }
    await client.write(f"{TASK_QUEUE}/enqueue", json.dumps(task, ensure_ascii=False))
    print(f"  [Enqueued] Task {task['id'][:8]}... ({task_type})")
    return task["id"]


async def dequeue_task(client):
    """Dequeue a task from the task queue."""
    try:
        data = await client.cat(f"{TASK_QUEUE}/dequeue")
        if isinstance(data, bytes):
            data = data.decode("utf-8", errors="ignore")
        if not data or data.strip() == "":
            return None
        return json.loads(data)
    except Exception as e:
        print(f"  [Dequeue] Empty or error: {e}")
        return None


async def process_task(client, task):
    """Process a task and write result."""
    task_id = task.get("id", "unknown")
    task_type = task.get("type", "unknown")
    payload = task.get("payload", {})

    print(f"  [Process] Task {task_id[:8]}... ({task_type})")

    # Simulate processing
    await asyncio.sleep(0.1)

    # Generate result
    result = {
        "task_id": task_id,
        "worker_id": WORKER_ID,
        "status": "completed",
        "type": task_type,
        "payload": payload,
        "result": f"Processed by {WORKER_ID}: {task_type} with {payload}",
    }

    # Write result
    await client.write(
        f"{RESULT_BASE}/{task_id}",
        json.dumps(result, ensure_ascii=False)
    )
    print(f"  [Result] Written to {RESULT_BASE}/{task_id[:8]}...")

    return result


async def setup_queues(client):
    """Mount memfs for task queue and results."""
    print("[Setup] Mounting memfs for task queue...")

    # Ensure directories exist
    try:
        await client.mkdir(TASK_QUEUE)
    except Exception:
        pass

    try:
        await client.mkdir(RESULT_BASE)
    except Exception:
        pass

    print("[Setup] Memfs mounted successfully")


async def cleanup(client):
    """Clean up task queue and results."""
    print("[Cleanup] Removing test data...")
    try:
        await client.rm(TASK_QUEUE, recursive=True)
    except Exception:
        pass
    try:
        await client.rm(RESULT_BASE, recursive=True)
    except Exception:
        pass
    print("[Cleanup] Done")


async def main():
    print("=" * 60)
    print("EVIF Agent Workflow Demo: Task Queue Worker")
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
        await setup_queues(client)
        print()

        # Enqueue 5 tasks
        print("[Enqueue] Submitting 5 tasks...")
        task_ids = []
        for i in range(5):
            task_id = await enqueue_task(
                client,
                task_type=["analyze", "process", "transform"][i % 3],
                payload={"input": f"data-{i}", "index": i}
            )
            task_ids.append(task_id)
        print()

        # Worker processes all tasks
        print(f"[Worker] {WORKER_ID} processing tasks...")
        results = []
        for i in range(5):
            task = await dequeue_task(client)
            if task:
                result = await process_task(client, task)
                results.append(result)
            else:
                print(f"  [Worker] No task in queue (attempt {i + 1})")
        print()

        # Summary
        print("=" * 60)
        print(f"Worker {WORKER_ID} completed {len(results)} tasks")
        print("=" * 60)

        # List results
        print("\n[Results] Generated files:")
        try:
            entries = await client.ls(RESULT_BASE)
            for entry in entries:
                if entry.is_file and entry.name.startswith("mem_"):
                    print(f"  - {entry.name}")
        except Exception as e:
            print(f"  (Could not list results: {e})")

        print()

        # Cleanup
        await cleanup(client)

    finally:
        await client.close()
        print("[Done] Connection closed")


if __name__ == "__main__":
    asyncio.run(main())