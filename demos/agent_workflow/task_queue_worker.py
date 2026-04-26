#!/usr/bin/env python3
"""
Task Queue Worker Demo — 展示 memfs + Python SDK 的 agent 协同模式

使用文件模拟队列：任务写入 /mem/tasks/pending/{task_id}
Worker 从中读取处理，结果写入 /mem/tasks/completed/{task_id}

Usage:
    python task_queue_worker.py
"""

import asyncio
import json
import uuid
from evif import EvifClient

WORKER_ID = "worker-1"
TASK_DIR = "/mem/tasks"
PENDING_DIR = f"{TASK_DIR}/pending"
COMPLETED_DIR = f"{TASK_DIR}/completed"


async def create_task(client, task_type: str, payload: dict) -> str:
    """创建一个任务到 pending 目录"""
    task_id = str(uuid.uuid4())
    task = {
        "id": task_id,
        "type": task_type,
        "payload": payload,
    }
    await client.write(f"{PENDING_DIR}/{task_id}", json.dumps(task, ensure_ascii=False))
    print(f"  [Created] Task {task_id[:8]}... ({task_type})")
    return task_id


async def get_pending_tasks(client) -> list:
    """获取所有待处理任务"""
    try:
        entries = await client.ls(PENDING_DIR)
        tasks = []
        for entry in entries:
            if entry.is_file:
                data = await client.cat(entry.path)
                if isinstance(data, bytes):
                    data = data.decode("utf-8", errors="ignore")
                tasks.append(json.loads(data))
        return tasks
    except Exception:
        return []


async def process_task(client, task: dict) -> dict:
    """处理任务并写入结果"""
    task_id = task.get("id", "unknown")
    task_type = task.get("type", "unknown")
    payload = task.get("payload", {})

    print(f"  [Process] Task {task_id[:8]}... ({task_type})")

    # 模拟处理
    await asyncio.sleep(0.1)

    # 生成结果
    result = {
        "task_id": task_id,
        "worker_id": WORKER_ID,
        "status": "completed",
        "type": task_type,
        "payload": payload,
        "result": f"Processed by {WORKER_ID}: {task_type} with {payload}",
    }

    # 写入结果
    await client.write(f"{COMPLETED_DIR}/{task_id}", json.dumps(result, ensure_ascii=False))
    print(f"  [Completed] {task_id[:8]}...")

    # 删除原始任务
    await client.rm(f"{PENDING_DIR}/{task_id}")

    return result


async def setup(client):
    """确保目录存在"""
    print("[Setup] Creating task directories...")
    try:
        await client.mkdir(TASK_DIR)
    except Exception:
        pass
    try:
        await client.mkdir(PENDING_DIR)
    except Exception:
        pass
    try:
        await client.mkdir(COMPLETED_DIR)
    except Exception:
        pass
    print("[Setup] Done")


async def main():
    print("=" * 60)
    print("EVIF Agent Workflow Demo: Task Queue Worker")
    print("=" * 60)
    print()

    # 创建客户端
    client = EvifClient(base_url="http://localhost:8081")

    try:
        await client.connect()
        print("[Connected] EVIF client connected\n")

        # 设置目录
        await setup(client)
        print()

        # 创建 5 个任务
        print("[Create] Submitting 5 tasks...")
        task_ids = []
        for i in range(5):
            task_id = await create_task(
                client,
                task_type=["analyze", "process", "transform"][i % 3],
                payload={"input": f"data-{i}", "index": i}
            )
            task_ids.append(task_id)
        print()

        # Worker 处理所有任务
        print(f"[Worker] {WORKER_ID} processing tasks...")
        results = []
        for _ in range(5):
            tasks = await get_pending_tasks(client)
            if tasks:
                task = tasks[0]
                result = await process_task(client, task)
                results.append(result)
            else:
                print(f"  [Worker] No pending tasks")
        print()

        # 摘要
        print("=" * 60)
        print(f"Worker {WORKER_ID} completed {len(results)} tasks")
        print("=" * 60)

        # 列出完成的结果
        print("\n[Results] Completed tasks:")
        completed = await get_pending_tasks(client)
        if not completed:
            print(f"  All {len(results)} tasks processed successfully!")
        else:
            print(f"  {len(completed)} tasks still pending")

        print()

    finally:
        await client.close()
        print("[Done] Connection closed")


if __name__ == "__main__":
    asyncio.run(main())
