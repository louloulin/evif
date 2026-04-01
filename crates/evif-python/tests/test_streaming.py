"""Tests for EVIF Streaming APIs (using httpx streaming + /api/v1/fs/stream endpoint)."""

import pytest
from unittest.mock import AsyncMock, MagicMock, ANY

from evif.client import EvifClient


def _make_client() -> EvifClient:
    """Create an EvifClient with a mocked _client so no HTTP calls are made."""
    client = EvifClient.__new__(EvifClient)
    client.base_url = "http://localhost:8080"
    client.timeout = 30.0
    client.max_retries = 3
    client.api_key = None
    client._client = MagicMock()  # satisfy _ensure_connected
    return client


class TestStreamRead:
    """Tests for stream_read using httpx streaming + /api/v1/fs/stream."""

    @pytest.mark.asyncio
    async def test_stream_read_returns_async_generator(self):
        """stream_read is an async generator that yields byte chunks."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200

        async def mock_aiter_bytes(chunk_size=65536):
            yield b"hello streaming world"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        result = client.stream_read("/test/file.txt")
        assert hasattr(result, "__aiter__"), "stream_read should return an async iterator"

    @pytest.mark.asyncio
    async def test_stream_read_yields_byte_chunks(self):
        """stream_read yields raw byte chunks from httpx streaming."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200

        async def mock_aiter_bytes(chunk_size=65536):
            # Simulate streaming: two chunks
            yield b"first chunk"
            yield b"second chunk"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        chunks = []
        async for chunk in client.stream_read("/test/file.bin"):
            chunks.append(chunk)

        assert chunks == [b"first chunk", b"second chunk"]

    @pytest.mark.asyncio
    async def test_stream_read_uses_streaming_endpoint(self):
        """Verify stream_read calls POST /api/v1/fs/stream with correct params."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200

        async def mock_aiter_bytes(chunk_size=65536):
            yield b"data"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        async for _ in client.stream_read("/path/to/file", offset=100, size=2048):
            pass

        # Verify stream() was called with the right JSON params
        # httpx stores resolved URL internally; verify via json kwarg
        call_kwargs = client._client.stream.call_args
        assert call_kwargs[0][0] == "POST"
        assert call_kwargs[1]["json"] == {
            "op": "read",
            "path": "/path/to/file",
            "offset": 100,
            "size": 2048,
        }

    @pytest.mark.asyncio
    async def test_stream_read_raises_on_404(self):
        """stream_read raises FileNotFoundError when file does not exist."""
        from evif.exceptions import FileNotFoundError

        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 404
        mock_response.aread = AsyncMock(return_value=b"Not found")

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        with pytest.raises(FileNotFoundError):
            async for _ in client.stream_read("/nonexistent/file.txt"):
                pass


class TestStreamWrite:
    """Tests for stream_write using raw byte streaming + /api/v1/fs/stream."""

    @pytest.mark.asyncio
    async def test_stream_write_with_string_content(self):
        """stream_write sends raw bytes to the streaming endpoint."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json = MagicMock(return_value={"bytes_written": 15})

        client._client.post = AsyncMock(return_value=mock_response)

        result = await client.stream_write("/test/file.txt", "hello streaming")

        call_kwargs = client._client.post.call_args
        # httpx post() takes path as first positional arg (method is implicit)
        assert call_kwargs[0][0] == "/api/v1/fs/stream"
        assert call_kwargs[1]["content"] == b"hello streaming"
        assert call_kwargs[1]["params"] == {
            "op": "write",
            "path": "/test/file.txt",
            "offset": -1,
        }
        assert result == 15

    @pytest.mark.asyncio
    async def test_stream_write_with_bytes_content(self):
        """stream_write handles raw bytes without encoding overhead."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json = MagicMock(return_value={"bytes_written": 5})

        client._client.post = AsyncMock(return_value=mock_response)

        result = await client.stream_write("/test/file.bin", b"bytes")

        call_kwargs = client._client.post.call_args
        assert call_kwargs[1]["content"] == b"bytes"
        assert result == 5

    @pytest.mark.asyncio
    async def test_stream_write_with_offset(self):
        """stream_write respects the offset parameter."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json = MagicMock(return_value={"bytes_written": 5})

        client._client.post = AsyncMock(return_value=mock_response)

        await client.stream_write("/test/file.txt", "append", offset=100)

        call_kwargs = client._client.post.call_args
        assert call_kwargs[1]["params"]["offset"] == 100


class TestStreamingIntegration:
    """Integration tests for streaming read + write round-trip."""

    @pytest.mark.asyncio
    async def test_stream_round_trip(self):
        """Verify stream_write then stream_read preserves data."""
        client = _make_client()

        # Mock write response
        write_response = MagicMock()
        write_response.status_code = 200
        write_response.json = MagicMock(return_value={"bytes_written": 10000})
        client._client.post = AsyncMock(return_value=write_response)

        # Mock read streaming response
        original_data = b"x" * 10000
        read_response = MagicMock()
        read_response.status_code = 200

        async def mock_aiter_bytes(chunk_size=65536):
            yield original_data

        read_response.aiter_bytes = mock_aiter_bytes

        read_stream_ctx = AsyncMock()
        read_stream_ctx.__aenter__ = AsyncMock(return_value=read_response)
        read_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=read_stream_ctx)

        # Write
        await client.stream_write("/test/large.txt", original_data)

        # Read
        chunks = []
        async for chunk in client.stream_read("/test/large.txt"):
            chunks.append(chunk)

        assert b"".join(chunks) == original_data
