"""Handle operations for EVIF SDK.

Handles provide stateful file access with seek, TTL, and renewal.
"""

from typing import Any, Dict, Optional

from .config import MemoryConfig


class HandleInfo:
    """Information about an open file handle."""

    def __init__(
        self,
        id: str,
        path: str = "",
        mode: str = "",
        offset: int = 0,
        ttl: int = 0,
        **kwargs: Any,
    ):
        self.id = id
        self.path = path
        self.mode = mode
        self.offset = offset
        self.ttl = ttl

    def __repr__(self) -> str:
        return f"HandleInfo({self.id!r}, path={self.path!r})"


class HandleOps:
    """Stateful file handle operations against the EVIF REST API.

    Access via ``EvifClient.handles``.
    """

    def __init__(self, config: MemoryConfig, request_fn: Any):
        self._config = config
        self._request = request_fn

    async def open(self, path: str, mode: str = "read") -> HandleInfo:
        """Open a file handle.

        Args:
            path: File path.
            mode: Open mode (``read``, ``write``, ``append``).

        Returns:
            ``HandleInfo`` for the opened handle.
        """
        body = {"path": path, "mode": mode}
        result = await self._request("POST", "/api/v1/handles/open", json=body)
        data = result.get("data", result)
        return HandleInfo(
            id=data.get("id", ""),
            path=data.get("path", path),
            mode=data.get("mode", mode),
            offset=data.get("offset", 0),
            ttl=data.get("ttl", 0),
        )

    async def get(self, handle_id: str) -> HandleInfo:
        """Get handle information.

        Args:
            handle_id: Handle ID.

        Returns:
            ``HandleInfo`` for the handle.
        """
        result = await self._request("GET", f"/api/v1/handles/{handle_id}")
        data = result.get("data", result)
        return HandleInfo(
            id=data.get("id", handle_id),
            path=data.get("path", ""),
            mode=data.get("mode", ""),
            offset=data.get("offset", 0),
            ttl=data.get("ttl", 0),
        )

    async def read(self, handle_id: str, size: int = -1) -> bytes:
        """Read from an open handle.

        Args:
            handle_id: Handle ID.
            size: Bytes to read (-1 for all remaining).

        Returns:
            Data read from the handle.
        """
        body: Dict[str, Any] = {"size": size}
        result = await self._request(
            "POST", f"/api/v1/handles/{handle_id}/read", json=body
        )
        data = result.get("data", result)
        content = data.get("content", data.get("data", ""))
        if isinstance(content, str):
            return content.encode("utf-8")
        if isinstance(content, bytes):
            return content
        return str(content).encode("utf-8")

    async def write(self, handle_id: str, data: bytes) -> int:
        """Write to an open handle.

        Args:
            handle_id: Handle ID.
            data: Bytes to write.

        Returns:
            Number of bytes written.
        """
        import base64

        body = {"content": base64.b64encode(data).decode("ascii")}
        result = await self._request(
            "POST", f"/api/v1/handles/{handle_id}/write", json=body
        )
        return result.get("data", {}).get("bytes_written", len(data))

    async def seek(self, handle_id: str, offset: int) -> int:
        """Seek to a position in an open handle.

        Args:
            handle_id: Handle ID.
            offset: Byte offset to seek to.

        Returns:
            New offset position.
        """
        body = {"offset": offset}
        result = await self._request(
            "POST", f"/api/v1/handles/{handle_id}/seek", json=body
        )
        return result.get("data", {}).get("offset", offset)

    async def sync(self, handle_id: str) -> bool:
        """Sync an open handle.

        Args:
            handle_id: Handle ID.

        Returns:
            True if successful.
        """
        await self._request("POST", f"/api/v1/handles/{handle_id}/sync")
        return True

    async def close(self, handle_id: str) -> bool:
        """Close an open handle.

        Args:
            handle_id: Handle ID.

        Returns:
            True if successful.
        """
        await self._request("POST", f"/api/v1/handles/{handle_id}/close")
        return True

    async def renew(self, handle_id: str) -> HandleInfo:
        """Renew handle TTL.

        Args:
            handle_id: Handle ID.

        Returns:
            Updated ``HandleInfo``.
        """
        result = await self._request("POST", f"/api/v1/handles/{handle_id}/renew")
        data = result.get("data", result)
        return HandleInfo(
            id=data.get("id", handle_id),
            path=data.get("path", ""),
            mode=data.get("mode", ""),
            offset=data.get("offset", 0),
            ttl=data.get("ttl", 0),
        )

    async def list_handles(self) -> list:
        """List all open handles.

        Returns:
            List of ``HandleInfo`` objects.
        """
        result = await self._request("GET", "/api/v1/handles")
        items = result.get("data", [])
        if isinstance(items, dict):
            items = items.get("handles", [])
        return [
            HandleInfo(
                id=item.get("id", ""),
                path=item.get("path", ""),
                mode=item.get("mode", ""),
                offset=item.get("offset", 0),
                ttl=item.get("ttl", 0),
            )
            for item in items
        ]

    async def stats(self) -> Dict[str, Any]:
        """Get handle statistics.

        Returns:
            Statistics dictionary.
        """
        result = await self._request("GET", "/api/v1/handles/stats")
        return result.get("data", {})
