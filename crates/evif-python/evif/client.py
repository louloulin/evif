"""EVIF Python Client - Core client implementation."""

import asyncio
from typing import Any, AsyncIterator, Iterator, Optional, Union
import httpx
from tenacity import (
    retry,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception_type,
)

from evif.exceptions import (
    EvifError,
    ClientError,
    AuthenticationError,
    FileNotFoundError as EvifFileNotFoundError,
    PermissionError,
    TimeoutError as EvifTimeoutError,
    ValidationError,
)
from evif.models import FileInfo, MountInfo, HealthStatus, HandleInfo
from evif.context import ContextApi
from evif.skill import SkillApi
from evif.memory import MemoryApi
from evif.queue import QueueApi


class EvifClient(ContextApi, SkillApi, MemoryApi, QueueApi):
    """Async EVIF client with retry and error handling."""

    def __init__(
        self,
        base_url: str = "http://localhost:8081",
        timeout: float = 30.0,
        max_retries: int = 3,
        api_key: Optional[str] = None,
    ):
        """Initialize EVIF client.

        Args:
            base_url: Base URL of EVIF server
            timeout: Request timeout in seconds
            max_retries: Maximum number of retry attempts
            api_key: Optional API key for authentication
        """
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.max_retries = max_retries
        self.api_key = api_key

        self._client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()

    async def connect(self):
        """Establish connection to EVIF server."""
        if self._client is None:
            headers = {}
            if self.api_key:
                headers["Authorization"] = f"Bearer {self.api_key}"

            self._client = httpx.AsyncClient(
                base_url=self.base_url,
                headers=headers,
                timeout=self.timeout,
            )

    async def close(self):
        """Close connection to EVIF server."""
        if self._client:
            await self._client.aclose()
            self._client = None

    def _ensure_connected(self):
        """Ensure client is connected."""
        if self._client is None:
            raise ClientError("Client not connected. Call connect() or use async context manager.")

    async def _request(
        self,
        method: str,
        path: str,
        **kwargs: Any,
    ) -> dict:
        """Make HTTP request with error handling and retry.

        Args:
            method: HTTP method (GET, POST, PUT, DELETE)
            path: API path
            **kwargs: Additional arguments for httpx request

        Returns:
            Response JSON as dict

        Raises:
            EvifError: On API errors
        """
        self._ensure_connected()

        @retry(
            stop=stop_after_attempt(self.max_retries),
            wait=wait_exponential(multiplier=1, min=1, max=10),
            retry=retry_if_exception_type((httpx.HTTPStatusError, httpx.ConnectError)),
        )
        async def _do_request() -> dict:
            response = await self._client.request(method, path, **kwargs)

            if response.status_code == 401:
                raise AuthenticationError()
            elif response.status_code == 403:
                raise PermissionError()
            elif response.status_code == 404:
                raise EvifFileNotFoundError(path)
            elif response.status_code >= 400:
                raise EvifError(
                    response.text, response.status_code
                )

            return response.json()

        return await _do_request()

    # ===== File Operations =====

    async def ls(
        self,
        path: str,
        limit: Optional[int] = None,
    ) -> list[FileInfo]:
        """List files in directory.

        Args:
            path: Directory path
            limit: Optional limit on number of files

        Returns:
            List of FileInfo objects
        """
        params = {"path": path}
        if limit:
            params["limit"] = limit

        data = await self._request("GET", "/api/v1/directories", params=params)
        return [FileInfo.from_dict(f) for f in data.get("files", [])]

    async def cat(
        self,
        path: str,
        offset: int = 0,
        size: int = 0,
    ) -> bytes:
        """Read file contents.

        Args:
            path: File path
            offset: Read offset in bytes
            size: Number of bytes to read (0 = entire file)

        Returns:
            File contents as bytes
        """
        data = await self._request("GET", "/api/v1/files", params={"path": path})
        content = data.get("content", data.get("data", ""))
        if isinstance(content, str):
            return content.encode("utf-8")
        return content

    async def write(
        self,
        path: str,
        content: Union[str, bytes],
        offset: int = -1,
    ) -> int:
        """Write content to file.

        Args:
            path: File path
            content: Content to write (string or bytes)
            offset: Write offset (-1 = append)

        Returns:
            Number of bytes written
        """
        if isinstance(content, str):
            content = content.encode("utf-8")

        data = await self._request(
            "PUT",
            "/api/v1/files",
            params={"path": path},
            json={"data": content.decode("utf-8", errors="ignore")},
        )
        return data.get("bytes_written", 0)

    async def mkdir(
        self,
        path: str,
        mode: int = 0o755,
    ) -> bool:
        """Create directory.

        Args:
            path: Directory path
            mode: Directory permissions (default 0o755)

        Returns:
            True if successful
        """
        await self._request(
            "POST",
            "/api/v1/directories",
            json={"path": path},
        )
        return True

    async def rm(
        self,
        path: str,
        recursive: bool = False,
    ) -> bool:
        """Remove file or directory.

        Args:
            path: File or directory path
            recursive: Recursively remove directory

        Returns:
            True if successful
        """
        await self._request("DELETE", "/api/v1/files", params={"path": path})
        return True

    async def stat(self, path: str) -> FileInfo:
        """Get file metadata.

        Args:
            path: File path

        Returns:
            FileInfo object
        """
        params = {"path": path}
        data = await self._request("POST", "/api/v1/fs/stat", json=params)
        return FileInfo.from_dict(data)

    async def mv(
        self,
        old_path: str,
        new_path: str,
    ) -> bool:
        """Move or rename file.

        Args:
            old_path: Source path
            new_path: Destination path

        Returns:
            True if successful
        """
        params = {"old_path": old_path, "new_path": new_path}
        await self._request("POST", "/api/v1/fs/rename", json=params)
        return True

    async def cp(
        self,
        src: str,
        dst: str,
    ) -> bool:
        """Copy file.

        Args:
            src: Source path
            dst: Destination path

        Returns:
            True if successful
        """
        # Read source
        content = await self.cat(src)

        # Write destination
        await self.write(dst, content)

        return True

    # ===== Plugin Operations =====

    async def mount(
        self,
        plugin: str,
        path: str,
        options: dict,
    ) -> bool:
        """Mount plugin at path.

        Args:
            plugin: Plugin name (e.g., "s3fs", "localfs")
            path: Mount path
            options: Plugin-specific options

        Returns:
            True if successful
        """
        params = {
            "plugin": plugin,
            "path": path,
            "options": options,
        }
        await self._request("POST", "/api/v1/mount/add", json=params)
        return True

    async def unmount(self, path: str) -> bool:
        """Unmount plugin at path.

        Args:
            path: Mount path

        Returns:
            True if successful
        """
        params = {"path": path}
        await self._request("POST", "/api/v1/mount/remove", json=params)
        return True

    async def mounts(self) -> list[MountInfo]:
        """List all mount points.

        Returns:
            List of MountInfo objects
        """
        data = await self._request("GET", "/api/v1/mounts")
        return [MountInfo.from_dict(m) for m in data.get("mounts", [])]

    # ===== Advanced Operations =====

    async def health(self) -> HealthStatus:
        """Get server health status.

        Returns:
            HealthStatus object
        """
        data = await self._request("GET", "/api/v1/health")
        return HealthStatus.from_dict(data)

    async def grep(
        self,
        path: str,
        pattern: str,
        recursive: bool = False,
    ) -> list[str]:
        """Search for pattern in files.

        Args:
            path: Search path
            pattern: Search pattern (supports regex)
            recursive: Search recursively

        Returns:
            List of matching lines
        """
        params = {
            "path": path,
            "pattern": pattern,
            "recursive": recursive,
        }
        data = await self._request("POST", "/api/v1/fs/grep", json=params)
        return data.get("matches", [])

    # ===== Handle Operations =====

    async def open_handle(
        self,
        path: str,
        flags: int = 1,  # READ_ONLY
        mode: int = 0o644,
        lease: int = 60,
    ) -> HandleInfo:
        """Open file handle.

        Args:
            path: File path
            flags: Open flags (bitmask)
            mode: File permissions
            lease: Lease duration in seconds

        Returns:
            HandleInfo object
        """
        from evif.file_handle import FileHandle

        params = {
            "path": path,
            "flags": flags,
            "mode": mode,
            "lease_seconds": lease,
        }
        data = await self._request("POST", "/api/v1/handles", json=params)
        return HandleInfo.from_dict(data)

    async def close_handle(self, handle_id: int) -> bool:
        """Close file handle.

        Args:
            handle_id: Handle ID

        Returns:
            True if successful
        """
        await self._request("DELETE", f"/api/v1/handles/{handle_id}")
        return True

    # ===== Streaming Operations =====

    async def stream_read(
        self,
        path: str,
        offset: int = 0,
        size: int = 0,
    ) -> AsyncIterator[bytes]:
        """Stream read file contents as raw bytes.

        Uses the dedicated streaming endpoint `/api/v1/fs/stream` which returns
        raw bytes without JSON/base64 wrapping. This enables true streaming for
        large files without buffering the entire response in memory.

        Args:
            path: File path
            offset: Read offset in bytes (default 0)
            size: Number of bytes to read, 0 = entire file (default 0)

        Yields:
            Raw byte chunks from the file

        Example:
            async for chunk in client.stream_read("/large/file.bin", chunk_size=65536):
                await file.write(chunk)
        """
        self._ensure_connected()

        params = {
            "op": "read",
            "path": path,
            "offset": offset,
            "size": size,
        }

        async with self._client.stream(
            "POST",
            "/api/v1/fs/stream",
            json=params,
            timeout=self.timeout,
        ) as response:
            if response.status_code == 401:
                raise AuthenticationError()
            elif response.status_code == 403:
                raise PermissionError()
            elif response.status_code == 404:
                raise EvifFileNotFoundError(path)
            elif response.status_code >= 400:
                text = await response.aread()
                raise EvifError(text.decode("utf-8", errors="ignore"), response.status_code)

            async for chunk in response.aiter_bytes(chunk_size=65536):
                if chunk:
                    yield chunk

    async def stream_write(
        self,
        path: str,
        content: Union[str, bytes],
        offset: int = -1,
    ) -> int:
        """Stream write content to a file.

        Sends content directly to the server without buffering the entire
        payload in memory. Suitable for large files or data streams.

        Args:
            path: File path
            content: Content to write (string or bytes). For true streaming
                     with large data, pass a string or bytes directly - httpx
                     will stream the body efficiently.
            offset: Write offset (-1 = append, default)

        Returns:
            Number of bytes written

        Example:
            # Write a large file from disk without loading it all into memory
            with open("large.bin", "rb") as f:
                data = f.read()
            result = await client.stream_write("/path/to/dest", data)
        """
        self._ensure_connected()

        if isinstance(content, str):
            body = content.encode("utf-8")
        else:
            body = content

        params = {
            "op": "write",
            "path": path,
            "offset": offset,
        }

        response = await self._client.post(
            "/api/v1/fs/stream",
            params=params,
            content=body,
            timeout=self.timeout,
        )

        if response.status_code == 401:
            raise AuthenticationError()
        elif response.status_code == 403:
            raise PermissionError()
        elif response.status_code == 404:
            raise EvifFileNotFoundError(path)
        elif response.status_code >= 400:
            raise EvifError(response.text, response.status_code)

        data = response.json()
        return data.get("bytes_written", 0)
