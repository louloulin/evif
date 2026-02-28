"""File handle implementation with async context manager support."""

from typing import Optional
from evif.client import EvifClient


class FileHandle:
    """File handle with async context manager support."""

    # Open flags
    READ_ONLY = 1 << 0
    WRITE_ONLY = 1 << 1
    READ_WRITE = 1 << 2
    CREATE = 1 << 3
    EXCLUSIVE = 1 << 4
    TRUNCATE = 1 << 5
    APPEND = 1 << 6
    NONBLOCK = 1 << 7

    def __init__(
        self,
        handle_id: int,
        path: str,
        client: EvifClient,
        flags: int = READ_ONLY,
    ):
        """Initialize file handle.

        Args:
            handle_id: Handle ID from server
            path: File path
            client: EVIF client instance
            flags: Open flags
        """
        self.id = handle_id
        self.path = path
        self._client = client
        self._flags = flags
        self._offset = 0
        self._closed = False

    async def __aenter__(self):
        """Async context manager entry."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit - auto close handle."""
        await self.close()
        return False

    async def read(self, size: int = 4096) -> bytes:
        """Read from file handle.

        Args:
            size: Number of bytes to read

        Returns:
            Read data as bytes
        """
        if self._closed:
            raise RuntimeError("Handle is closed")

        data = await self._client._request(
            "POST",
            f"/api/v1/handles/{self.id}/read",
            json={"offset": self._offset, "size": size},
        )

        content = data.get("data", "")
        if isinstance(content, str):
            # Try hex decoding first
            try:
                content_bytes = bytes.fromhex(content)
            except ValueError:
                # Fall back to UTF-8
                content_bytes = content.encode("utf-8")
        else:
            content_bytes = content

        self._offset += len(content_bytes)
        return content_bytes

    async def write(self, data: bytes) -> int:
        """Write to file handle.

        Args:
            data: Data to write

        Returns:
            Number of bytes written
        """
        if self._closed:
            raise RuntimeError("Handle is closed")

        # Convert to hex for transport
        data_hex = data.hex() if isinstance(data, bytes) else data.encode().hex()

        result = await self._client._request(
            "POST",
            f"/api/v1/handles/{self.id}/write",
            json={"offset": self._offset, "data": data_hex},
        )

        bytes_written = result.get("bytes_written", len(data))
        self._offset += bytes_written
        return bytes_written

    async def seek(self, offset: int, whence: int = 0) -> int:
        """Seek to position in file.

        Args:
            offset: Offset in bytes
            whence: 0 = from start, 1 = from current, 2 = from end

        Returns:
            New position
        """
        if self._closed:
            raise RuntimeError("Handle is closed")

        if whence == 0:  # SEEK_SET
            self._offset = offset
        elif whence == 1:  # SEEK_CUR
            self._offset += offset
        elif whence == 2:  # SEEK_END
            # Get file size first
            info = await self._client.stat(self.path)
            self._offset = info.size + offset
        else:
            raise ValueError(f"Invalid whence value: {whence}")

        return self._offset

    async def tell(self) -> int:
        """Get current position.

        Returns:
            Current offset in bytes
        """
        return self._offset

    async def flush(self) -> bool:
        """Flush file handle.

        Returns:
            True if successful
        """
        if self._closed:
            raise RuntimeError("Handle is closed")

        await self._client._request("POST", f"/api/v1/handles/{self.id}/flush")
        return True

    async def renew_lease(self, lease_seconds: int = 60) -> bool:
        """Renew handle lease.

        Args:
            lease_seconds: New lease duration

        Returns:
            True if successful
        """
        if self._closed:
            raise RuntimeError("Handle is closed")

        await self._client._request(
            "POST",
            f"/api/v1/handles/{self.id}/renew",
            json={"lease_seconds": lease_seconds},
        )
        return True

    async def close(self) -> bool:
        """Close file handle.

        Returns:
            True if successful
        """
        if self._closed:
            return True

        await self._client.close_handle(self.id)
        self._closed = True
        return True

    @property
    def closed(self) -> bool:
        """Check if handle is closed."""
        return self._closed

    def __repr__(self) -> str:
        """String representation."""
        return f"FileHandle(id={self.id}, path='{self.path}', offset={self._offset})"
