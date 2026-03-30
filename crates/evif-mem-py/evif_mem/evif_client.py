"""Unified EVIF Client - Combines filesystem, plugin, handle, and memory operations."""

import asyncio
from typing import Any, Dict

import httpx

from .config import MemoryConfig
from .client import EvifMemoryClient
from .filesystem import FilesystemOps
from .plugins import PluginOps
from .handles import HandleOps


class EvifClient:
    """Unified client for the EVIF REST API.

    Provides access to all EVIF features through a single client:

    - ``client.fs`` — Filesystem operations (read, write, ls, mkdir, rm, mv, cp, stat)
    - ``client.plugins`` — Plugin management (mount, unmount, list)
    - ``client.handles`` — Stateful file handles (open, read, write, seek, close)
    - ``client.memory`` — Memory platform (create, search, query memories)

    Example::

        from evif_mem import EvifClient, EvifConfig

        config = EvifConfig(api_url="http://localhost:8080")
        async with EvifClient(config) as client:
            # Filesystem operations
            await client.fs.mkdir("/mydir")
            await client.fs.write_text("/mydir/file.txt", "Hello!")
            content = await client.fs.read_text("/mydir/file.txt")

            # Plugin management
            mounts = await client.plugins.list_mounts()
            await client.plugins.mount("memfs", "/test")

            # Memory platform
            memory = await client.memory.create_memory("Important note")
            results = await client.memory.search_memories("note")
    """

    def __init__(self, config: MemoryConfig):
        self._config = config
        self._http = httpx.AsyncClient(
            base_url=config.api_url,
            timeout=config.timeout,
            headers=self._build_headers(),
        )
        self.fs = FilesystemOps(config, self._request)
        self.plugins = PluginOps(config, self._request)
        self.handles = HandleOps(config, self._request)
        self.memory = EvifMemoryClient(config)
        # Share the HTTP client with the memory client
        self.memory._client = self._http

    def _build_headers(self) -> Dict[str, str]:
        headers = {
            "Content-Type": "application/json",
            "User-Agent": "evif-python/0.2.0",
        }
        if self._config.api_key:
            headers["Authorization"] = f"Bearer {self._config.api_key}"
        return headers

    async def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        """Make an HTTP request with retry logic."""
        retries = 0
        last_error = None

        while retries <= self._config.max_retries:
            try:
                response = await self._http.request(method, path, **kwargs)
                response.raise_for_status()
                return response.json()
            except httpx.HTTPError as e:
                last_error = e
                retries += 1
                if retries <= self._config.max_retries:
                    await asyncio.sleep(0.5 * retries)

        raise RuntimeError(
            f"Request failed after {self._config.max_retries} retries: {last_error}"
        )

    async def health(self) -> Dict[str, Any]:
        """Check server health.

        Returns:
            Health status dictionary.
        """
        result = await self._request("GET", "/api/v1/health")
        return result.get("data", result)

    async def close(self) -> None:
        """Close the client and release resources."""
        await self._http.aclose()

    async def __aenter__(self) -> "EvifClient":
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.close()


# Backward-compatible alias
EvifConfig = MemoryConfig
