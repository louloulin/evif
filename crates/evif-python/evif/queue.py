"""EVIF Queue/Pipe API - Queue and pipe operations for agent coordination."""

import json
from typing import Optional, Union


class QueueApi:
    """Mixin providing Queue and Pipe API operations for EvifClient.

    Queue API provides FIFO queue semantics using the EVIF filesystem.
    Pipe API provides bidirectional communication between agents.

    The queuefs plugin exposes operations via filesystem paths:
    - /queue_name/enqueue - write to add to queue
    - /queue_name/dequeue - read to pop from queue
    - /queue_name/peek - read without removing
    - /queue_name/size - read current queue size

    The pipefs plugin exposes:
    - /pipe_name/input - write input data
    - /pipe_name/output - read output data
    - /pipe_name/status - read/write status (idle/running/complete/error)
    - /pipe_name/assignee - read/write assigned worker
    - /pipe_name/timeout - read/write timeout in seconds
    """

    # ===== Queue Operations =====

    async def queue_push(
        self,
        queue_name: str,
        data: Union[str, dict, bytes],
        priority: int = 0,
    ) -> bool:
        """Push data to a queue.

        Writes to /{queue_name}/enqueue path. The queuefs plugin
        handles the actual queue semantics.

        Args:
            queue_name: Name of the queue (will be prefixed with /queues/)
            data: Data to enqueue (will be JSON-encoded if dict)
            priority: Priority level (higher = more urgent)

        Returns:
            True if successful
        """
        import uuid

        queue_path = f"/queues/{queue_name}"

        # Serialize data
        if isinstance(data, dict):
            payload = json.dumps(data, ensure_ascii=False)
        elif isinstance(data, bytes):
            payload = data.decode("utf-8", errors="ignore")
        else:
            payload = str(data)

        # Create queue entry with metadata
        entry = {
            "id": str(uuid.uuid4()),
            "data": payload,
            "priority": priority,
        }

        # Write to enqueue path
        await self.write(f"{queue_path}/enqueue", json.dumps(entry, ensure_ascii=False))
        return True

    async def queue_pop(self, queue_name: str, timeout: int = 0) -> Optional[dict]:
        """Pop data from a queue (blocking with optional timeout).

        Reads from /{queue_name}/dequeue path. Returns None if queue
        is empty (non-blocking) or after timeout.

        Args:
            queue_name: Name of the queue
            timeout: Seconds to wait (0 = non-blocking)

        Returns:
            Dict with 'id' and 'data' keys, or None if empty
        """
        queue_path = f"/queues/{queue_name}"

        try:
            content = await self.cat(f"{queue_path}/dequeue")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")

            if not content or content.strip() == "":
                return None

            return json.loads(content)
        except Exception:
            return None

    async def queue_peek(self, queue_name: str) -> Optional[dict]:
        """Peek at the next item without removing it.

        Args:
            queue_name: Name of the queue

        Returns:
            Dict with 'id' and 'data' keys, or None if empty
        """
        queue_path = f"/queues/{queue_name}"

        try:
            content = await self.cat(f"{queue_path}/peek")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")

            if not content or content.strip() == "":
                return None

            return json.loads(content)
        except Exception:
            return None

    async def queue_size(self, queue_name: str) -> int:
        """Get the current size of a queue.

        Reads from /{queue_name}/size path.

        Args:
            queue_name: Name of the queue

        Returns:
            Number of items in queue
        """
        queue_path = f"/queues/{queue_name}"

        try:
            content = await self.cat(f"{queue_path}/size")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")

            return int(content.strip())
        except Exception:
            return 0

    async def queue_clear(self, queue_name: str) -> bool:
        """Clear all items from a queue.

        Args:
            queue_name: Name of the queue

        Returns:
            True if successful
        """
        queue_path = f"/queues/{queue_name}"

        try:
            entries = await self.ls(queue_path)
            for entry in entries:
                if entry.name not in (".size", ".head", ".tail"):
                    try:
                        await self.rm(f"{queue_path}/{entry.name}")
                    except Exception:
                        pass
            return True
        except Exception:
            return False

    # ===== Pipe Operations =====

    async def pipe_write(self, pipe_name: str, data: Union[str, dict, bytes]) -> bool:
        """Write data to a pipe's input.

        Writes to /{pipe_name}/input path. The pipefs plugin
        handles the bidirectional communication semantics.

        Args:
            pipe_name: Name of the pipe (will be prefixed with /pipes/)
            data: Data to write (will be JSON-encoded if dict)

        Returns:
            True if successful
        """
        pipe_path = f"/pipes/{pipe_name}"

        # Serialize data
        if isinstance(data, dict):
            payload = json.dumps(data, ensure_ascii=False)
        elif isinstance(data, bytes):
            payload = data.decode("utf-8", errors="ignore")
        else:
            payload = str(data)

        await self.write(f"{pipe_path}/input", payload)
        return True

    async def pipe_read(self, pipe_name: str, clear: bool = False) -> Optional[dict]:
        """Read data from a pipe's output.

        Reads from /{pipe_name}/output path. Optionally clears
        the output after reading.

        Args:
            pipe_name: Name of the pipe
            clear: Whether to clear the output after reading

        Returns:
            Dict with output data, or None if empty
        """
        pipe_path = f"/pipes/{pipe_name}"

        try:
            content = await self.cat(f"{pipe_path}/output")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")

            if not content or content.strip() == "":
                return None

            result = json.loads(content) if content.strip().startswith("{") else {"data": content}

            if clear:
                await self.write(f"{pipe_path}/output", "")

            return result
        except Exception:
            return None

    async def pipe_status(self, pipe_name: str) -> dict:
        """Get pipe status and metadata.

        Reads from /{pipe_name}/status, /{pipe_name}/assignee, etc.

        Args:
            pipe_name: Name of the pipe

        Returns:
            Dict with status, assignee, timeout, etc.
        """
        pipe_path = f"/pipes/{pipe_name}"

        status = {"name": pipe_name, "status": "unknown", "assignee": None, "timeout": 60}

        try:
            # Read status
            content = await self.cat(f"{pipe_path}/status")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")
            status["status"] = content.strip() or "idle"
        except Exception:
            pass

        try:
            # Read assignee
            content = await self.cat(f"{pipe_path}/assignee")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")
            if content.strip():
                status["assignee"] = content.strip()
        except Exception:
            pass

        try:
            # Read timeout
            content = await self.cat(f"{pipe_path}/timeout")
            if isinstance(content, bytes):
                content = content.decode("utf-8", errors="ignore")
            if content.strip():
                status["timeout"] = int(content.strip())
        except Exception:
            pass

        return status

    async def pipe_set_status(
        self,
        pipe_name: str,
        status: str,
        assignee: Optional[str] = None,
    ) -> bool:
        """Set pipe status and optionally assignee.

        Args:
            pipe_name: Name of the pipe
            status: Status (idle, running, complete, error)
            assignee: Optional worker ID

        Returns:
            True if successful
        """
        pipe_path = f"/pipes/{pipe_name}"

        await self.write(f"{pipe_path}/status", status)
        if assignee is not None:
            await self.write(f"{pipe_path}/assignee", assignee)

        return True

    async def pipe_clear(self, pipe_name: str) -> bool:
        """Clear a pipe (reset input and output).

        Args:
            pipe_name: Name of the pipe

        Returns:
            True if successful
        """
        pipe_path = f"/pipes/{pipe_name}"

        try:
            await self.write(f"{pipe_path}/input", "")
            await self.write(f"{pipe_path}/output", "")
            await self.write(f"{pipe_path}/status", "idle")
            await self.write(f"{pipe_path}/assignee", "")
            return True
        except Exception:
            return False