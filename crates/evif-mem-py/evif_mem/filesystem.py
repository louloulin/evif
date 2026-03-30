"""Filesystem operations for EVIF SDK."""

from typing import List, Optional, Dict, Any

from .config import MemoryConfig


class FileInfo:
    """File or directory metadata."""

    def __init__(
        self,
        name: str,
        size: int = 0,
        mode: int = 0,
        is_dir: bool = False,
        modified: Optional[str] = None,
        **kwargs: Any,
    ):
        self.name = name
        self.size = size
        self.mode = mode
        self.is_dir = is_dir
        self.modified = modified

    def __repr__(self) -> str:
        kind = "dir" if self.is_dir else "file"
        return f"FileInfo({self.name!r}, {kind}, {self.size}B)"


class FilesystemOps:
    """Filesystem operations against the EVIF REST API.

    This class is not meant to be instantiated directly.
    Use ``EvifClient.fs`` to access these methods.
    """

    def __init__(self, config: MemoryConfig, request_fn: Any):
        self._config = config
        self._request = request_fn

    async def read(self, path: str) -> bytes:
        """Read file contents.

        Args:
            path: File path (e.g. ``/memfs/hello.txt``)

        Returns:
            File contents as bytes.
        """
        result = await self._request("GET", "/api/v1/files", params={"path": path})
        data = result.get("data", result)
        if isinstance(data, dict):
            content = data.get("content", data.get("data", ""))
        else:
            content = data
        if isinstance(content, str):
            return content.encode("utf-8")
        if isinstance(content, bytes):
            return content
        return str(content).encode("utf-8")

    async def read_text(self, path: str, encoding: str = "utf-8") -> str:
        """Read file contents as text.

        Args:
            path: File path.
            encoding: Text encoding (default utf-8).

        Returns:
            File contents as string.
        """
        data = await self.read(path)
        return data.decode(encoding)

    async def write(self, path: str, data: bytes, append: bool = False) -> int:
        """Write data to a file.

        Args:
            path: File path.
            data: Bytes to write.
            append: Append to existing file instead of overwriting.

        Returns:
            Number of bytes written.
        """
        import base64

        body: Dict[str, Any] = {
            "path": path,
            "content": base64.b64encode(data).decode("ascii"),
            "append": append,
        }
        result = await self._request("PUT", "/api/v1/files", json=body)
        return result.get("data", {}).get("bytes_written", len(data))

    async def write_text(
        self, path: str, text: str, encoding: str = "utf-8", append: bool = False
    ) -> int:
        """Write text to a file.

        Args:
            path: File path.
            text: Text content to write.
            encoding: Text encoding.
            append: Append instead of overwrite.

        Returns:
            Number of bytes written.
        """
        return await self.write(path, text.encode(encoding), append=append)

    async def ls(self, path: str = "/") -> List[FileInfo]:
        """List directory contents.

        Args:
            path: Directory path (default ``/``).

        Returns:
            List of ``FileInfo`` objects.
        """
        result = await self._request("GET", "/api/v1/directories", params={"path": path})
        items = result.get("data", [])
        if isinstance(items, dict):
            items = items.get("entries", items.get("children", []))
        return [
            FileInfo(
                name=item.get("name", ""),
                size=item.get("size", 0),
                mode=item.get("mode", 0),
                is_dir=item.get("is_dir", False),
                modified=item.get("modified"),
            )
            for item in items
        ]

    async def mkdir(self, path: str, parents: bool = False) -> bool:
        """Create a directory.

        Args:
            path: Directory path.
            parents: Create parent directories as needed.

        Returns:
            True if successful.
        """
        body: Dict[str, Any] = {"path": path}
        if parents:
            body["parents"] = True
        await self._request("POST", "/api/v1/directories", json=body)
        return True

    async def rm(self, path: str, recursive: bool = False) -> bool:
        """Remove a file or directory.

        Args:
            path: Path to remove.
            recursive: Remove directories recursively.

        Returns:
            True if successful.
        """
        params: Dict[str, Any] = {"path": path}
        if recursive:
            params["recursive"] = True
        await self._request("DELETE", "/api/v1/files", params=params)
        return True

    async def stat(self, path: str) -> FileInfo:
        """Get file/directory metadata.

        Args:
            path: Path to inspect.

        Returns:
            ``FileInfo`` with metadata.
        """
        result = await self._request("GET", "/api/v1/stat", params={"path": path})
        data = result.get("data", result)
        return FileInfo(
            name=data.get("name", ""),
            size=data.get("size", 0),
            mode=data.get("mode", 0),
            is_dir=data.get("is_dir", False),
            modified=data.get("modified"),
        )

    async def mv(self, src: str, dst: str) -> bool:
        """Move or rename a file/directory.

        Args:
            src: Source path.
            dst: Destination path.

        Returns:
            True if successful.
        """
        await self._request("POST", "/api/v1/rename", json={"src": src, "dst": dst})
        return True

    async def cp(self, src: str, dst: str) -> bool:
        """Copy a file.

        Args:
            src: Source path.
            dst: Destination path.

        Returns:
            True if successful.
        """
        await self._request("POST", "/api/v1/batch/copy", json={"src": src, "dst": dst})
        return True

    async def touch(self, path: str) -> bool:
        """Create an empty file or update modification time.

        Args:
            path: File path.

        Returns:
            True if successful.
        """
        await self._request("POST", "/api/v1/touch", json={"path": path})
        return True

    async def digest(self, path: str, algorithm: str = "sha256") -> str:
        """Compute file digest/checksum.

        Args:
            path: File path.
            algorithm: Hash algorithm (default sha256).

        Returns:
            Hex digest string.
        """
        result = await self._request(
            "POST", "/api/v1/digest", json={"path": path, "algorithm": algorithm}
        )
        return result.get("data", {}).get("digest", "")

    async def grep(
        self, path: str, pattern: str, recursive: bool = False
    ) -> List[Dict[str, Any]]:
        """Search file contents.

        Args:
            path: Path to search.
            pattern: Regex pattern.
            recursive: Search recursively.

        Returns:
            List of match results.
        """
        body: Dict[str, Any] = {
            "path": path,
            "pattern": pattern,
            "recursive": recursive,
        }
        result = await self._request("POST", "/api/v1/grep", json=body)
        return result.get("data", [])
