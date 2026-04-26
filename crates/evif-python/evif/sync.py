"""EVIF Sync Client - Synchronous wrapper around EvifClient."""

import asyncio
from typing import Any, Optional, Union

from evif.client import EvifClient
from evif.exceptions import (
    EvifError,
    ClientError,
    AuthenticationError,
    FileNotFoundError,
    PermissionError,
    TimeoutError,
)


def _run_async(coro):
    """Run async coroutine in sync context using a persistent event loop."""
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None

    if loop and loop.is_running():
        # If there's already a running loop (e.g., in Jupyter), create a new thread
        import concurrent.futures
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(asyncio.run, coro)
            return future.result()
    else:
        # Use a persistent loop to avoid httpx connection pool issues
        if not hasattr(_run_async, '_loop') or _run_async._loop is None or _run_async._loop.is_closed():
            _run_async._loop = asyncio.new_event_loop()
        return _run_async._loop.run_until_complete(coro)

_run_async._loop = None


def Client(
    base_url: str = "http://localhost:8081",
    api_key: Optional[str] = None,
) -> "SyncEvifClient":
    """Create a synchronous EVIF client.

    This is the main entry point for the Python SDK. It provides a
    synchronous API that wraps the async EvifClient internally.

    Args:
        base_url: Base URL of EVIF server (default: http://localhost:8081)
        api_key: Optional API key for authentication

    Returns:
        SyncEvifClient instance

    Example:
        from evif import Client

        client = Client("http://localhost:8081", api_key="write-key")
        health = client.health()
        print(health)
    """
    return SyncEvifClient(base_url=base_url, api_key=api_key)


class SyncEvifClient:
    """Synchronous wrapper for EvifClient.

    Provides the same API as EvifClient but with synchronous methods.
    Uses a persistent event loop to avoid httpx connection pool issues.

    Example:
        from evif import Client

        client = Client("http://localhost:8081", api_key="write-key")
        print(client.health())
        client.ls("/data")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8081",
        timeout: float = 30.0,
        max_retries: int = 3,
        api_key: Optional[str] = None,
        auto_connect: bool = True,
    ):
        """Initialize sync EVIF client.

        Args:
            base_url: Base URL of EVIF server
            timeout: Request timeout in seconds
            max_retries: Maximum number of retry attempts
            api_key: Optional API key for authentication
            auto_connect: Whether to auto-connect (default: True)
        """
        self._client = EvifClient(
            base_url=base_url,
            timeout=timeout,
            max_retries=max_retries,
            api_key=api_key,
        )
        # Auto-connect for sync usage
        if auto_connect:
            _run_async(self._client.connect())

    def _async_method(self, coro):
        """Wrapper to run async methods synchronously."""
        return _run_async(coro)

    def health(self) -> dict:
        """Get server health status."""
        return _run_async(self._client.health())

    def ls(self, path: str, limit: Optional[int] = None) -> list:
        """List files in directory."""
        return _run_async(self._client.ls(path, limit))

    def cat(self, path: str, offset: int = 0, size: int = 0) -> bytes:
        """Read file contents."""
        return _run_async(self._client.cat(path, offset, size))

    def write(self, path: str, content: Union[str, bytes], offset: int = -1) -> int:
        """Write content to file."""
        return _run_async(self._client.write(path, content, offset))

    def mkdir(self, path: str, mode: int = 0o755) -> bool:
        """Create directory."""
        return _run_async(self._client.mkdir(path, mode))

    def rm(self, path: str, recursive: bool = False) -> bool:
        """Remove file or directory."""
        return _run_async(self._client.rm(path, recursive))

    def stat(self, path: str) -> dict:
        """Get file metadata."""
        return _run_async(self._client.stat(path))

    def mv(self, old_path: str, new_path: str) -> bool:
        """Move or rename file."""
        return _run_async(self._client.mv(old_path, new_path))

    def cp(self, src: str, dst: str) -> bool:
        """Copy file."""
        return _run_async(self._client.cp(src, dst))

    def mount(self, plugin: str, path: str, options: dict) -> bool:
        """Mount plugin at path."""
        return _run_async(self._client.mount(plugin, path, options))

    def unmount(self, path: str) -> bool:
        """Unmount plugin at path."""
        return _run_async(self._client.unmount(path))

    def mounts(self) -> list:
        """List all mount points."""
        return _run_async(self._client.mounts())

    def plugins(self) -> list:
        """List available plugins."""
        return _run_async(self._client.plugins())

    def grep(self, path: str, pattern: str, recursive: bool = False) -> list:
        """Search for pattern in files."""
        return _run_async(self._client.grep(path, pattern, recursive))

    def open_handle(self, path: str, flags: int = 1, mode: int = 0o644, lease: int = 60) -> dict:
        """Open file handle."""
        return _run_async(self._client.open_handle(path, flags, mode, lease))

    def close_handle(self, handle_id: int) -> bool:
        """Close file handle."""
        return _run_async(self._client.close_handle(handle_id))

    def stream_read(self, path: str, offset: int = 0, size: int = 0):
        """Stream read file - returns async generator, use async client for streaming."""
        raise NotImplementedError("Use async EvifClient for streaming operations")

    def stream_write(self, path: str, content: Union[str, bytes], offset: int = -1) -> int:
        """Stream write content to a file."""
        return _run_async(self._client.stream_write(path, content, offset))

    # Context API sync wrappers
    def context_read(self, path: str) -> str:
        """Read a context file from /context/{path}."""
        return _run_async(self._client.context_read(path))

    def context_write(self, path: str, content: str) -> int:
        """Write to a context file at /context/{path}."""
        return _run_async(self._client.context_write(path, content))

    def context_list(self, layer: str = "") -> list:
        """List files in context layer."""
        return _run_async(self._client.context_list(layer))

    def context_current(self) -> str:
        """Read L0 current context."""
        return _run_async(self._client.context_current())

    def context_update_current(self, context: str) -> int:
        """Update L0 current context."""
        return _run_async(self._client.context_update_current(context))

    def context_decisions(self) -> str:
        """Read L1 decisions."""
        return _run_async(self._client.context_decisions())

    def context_add_decision(self, decision: str) -> int:
        """Append a decision to L1/decisions.md."""
        return _run_async(self._client.context_add_decision(decision))

    def context_recent_ops(self) -> list:
        """Read L0 recent operations."""
        return _run_async(self._client.context_recent_ops())

    def context_search(self, query: str, layer: Optional[str] = None) -> list:
        """Search context files using grep."""
        return _run_async(self._client.context_search(query, layer))

    def context_meta(self) -> dict:
        """Read context metadata."""
        return _run_async(self._client.context_meta())

    def context_knowledge(self, name: str) -> str:
        """Read a L2 knowledge file."""
        return _run_async(self._client.context_knowledge(name))

    def context_add_knowledge(self, name: str, content: str) -> int:
        """Write a L2 knowledge file."""
        return _run_async(self._client.context_add_knowledge(name, content))

    # Skill API sync wrappers
    def skill_discover(self, domain: str = "", limit: int = 20) -> list:
        """Discover skills by domain."""
        return _run_async(self._client.skill_discover(domain, limit))

    def skill_read(self, name: str) -> str:
        """Read skill content by name."""
        return _run_async(self._client.skill_read(name))

    def skill_execute(self, name: str, context: Optional[dict] = None) -> dict:
        """Execute a skill."""
        return _run_async(self._client.skill_execute(name, context))

    def skill_register(self, name: str, content: str, domain: str = "") -> bool:
        """Register a new skill."""
        return _run_async(self._client.skill_register(name, content, domain))

    def skill_match(self, query: str, limit: int = 5) -> list:
        """Find skills matching a query."""
        return _run_async(self._client.skill_match(query, limit))

    def skill_remove(self, name: str) -> bool:
        """Remove a skill."""
        return _run_async(self._client.skill_remove(name))

    # Memory API sync wrappers (new)
    def memory_store(self, content: str, modality: str = "text", metadata: Optional[dict] = None) -> dict:
        """Store content in memory."""
        return _run_async(self._client.memory_store(content, modality, metadata))

    def memory_search(self, query: str, limit: int = 10) -> list:
        """Search memory content."""
        return _run_async(self._client.memory_search(query, limit))

    def memory_list(self, modality: Optional[str] = None, limit: int = 100) -> list:
        """List memory entries."""
        return _run_async(self._client.memory_list(modality, limit))

    # Queue/Pipe API sync wrappers (new)
    def queue_push(self, queue_name: str, data: Union[str, dict, bytes]) -> bool:
        """Push data to queue."""
        return _run_async(self._client.queue_push(queue_name, data))

    def queue_pop(self, queue_name: str) -> Optional[dict]:
        """Pop data from queue."""
        return _run_async(self._client.queue_pop(queue_name))

    def queue_size(self, queue_name: str) -> int:
        """Get queue size."""
        return _run_async(self._client.queue_size(queue_name))

    def pipe_write(self, pipe_name: str, data: Union[str, dict, bytes]) -> bool:
        """Write data to pipe input."""
        return _run_async(self._client.pipe_write(pipe_name, data))

    def pipe_read(self, pipe_name: str) -> Optional[dict]:
        """Read data from pipe output."""
        return _run_async(self._client.pipe_read(pipe_name))

    def pipe_status(self, pipe_name: str) -> dict:
        """Get pipe status."""
        return _run_async(self._client.pipe_status(pipe_name))


def Client(
    base_url: str = "http://localhost:8081",
    api_key: Optional[str] = None,
) -> SyncEvifClient:
    """Create a synchronous EVIF client.

    This is the main entry point for the Python SDK. It provides a
    synchronous API that wraps the async EvifClient internally.

    Args:
        base_url: Base URL of EVIF server (default: http://localhost:8081)
        api_key: Optional API key for authentication

    Returns:
        SyncEvifClient instance

    Example:
        from evif import Client

        client = Client("http://localhost:8081", api_key="write-key")
        health = client.health()
        print(health)
    """
    return SyncEvifClient(base_url=base_url, api_key=api_key)
