"""Tests for EVIF Context and Skill APIs."""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch, ANY

from evif.client import EvifClient
from evif.models import FileInfo


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_client() -> EvifClient:
    """Create an EvifClient with a mocked _request so no HTTP calls are made."""
    client = EvifClient.__new__(EvifClient)
    client.base_url = "http://localhost:8080"
    client.timeout = 30.0
    client.max_retries = 3
    client.api_key = None
    client._client = MagicMock()  # satisfy _ensure_connected
    return client


def _file_info(name: str, is_dir: bool = False) -> FileInfo:
    """Create a minimal FileInfo instance."""
    return FileInfo(
        name=name,
        path=f"/{name}",
        size=0,
        mode=0o755 if is_dir else 0o644,
        mtime=0.0,
        is_dir=is_dir,
        is_file=not is_dir,
    )


# ===========================================================================
# Context API tests
# ===========================================================================


class TestContextRead:
    """Tests for context_read."""

    @pytest.mark.asyncio
    async def test_returns_string_from_bytes(self):
        client = _make_client()
        client.cat = AsyncMock(return_value=b"hello context")

        result = await client.context_read("L0/current")

        client.cat.assert_awaited_once_with("/context/L0/current")
        assert result == "hello context"

    @pytest.mark.asyncio
    async def test_returns_string_when_cat_returns_string(self):
        client = _make_client()
        client.cat = AsyncMock(return_value="string data")

        result = await client.context_read("L0/current")

        assert result == "string data"


class TestContextWrite:
    @pytest.mark.asyncio
    async def test_delegates_to_write(self):
        client = _make_client()
        client.write = AsyncMock(return_value=42)

        result = await client.context_write("L0/current", "new value")

        client.write.assert_awaited_once_with("/context/L0/current", "new value")
        assert result == 42


class TestContextList:
    @pytest.mark.asyncio
    async def test_list_root(self):
        client = _make_client()
        expected = [_file_info("L0", is_dir=True), _file_info("L1", is_dir=True)]
        client.ls = AsyncMock(return_value=expected)

        result = await client.context_list()

        client.ls.assert_awaited_once_with("/context")
        assert result == expected

    @pytest.mark.asyncio
    async def test_list_layer(self):
        client = _make_client()
        expected = [_file_info("current"), _file_info("recent_ops")]
        client.ls = AsyncMock(return_value=expected)

        result = await client.context_list("L0")

        client.ls.assert_awaited_once_with("/context/L0")
        assert result == expected


class TestContextCurrent:
    @pytest.mark.asyncio
    async def test_reads_l0_current(self):
        client = _make_client()
        client.context_read = AsyncMock(return_value="my context")

        result = await client.context_current()

        client.context_read.assert_awaited_once_with("L0/current")
        assert result == "my context"


class TestContextUpdateCurrent:
    @pytest.mark.asyncio
    async def test_writes_l0_current(self):
        client = _make_client()
        client.context_write = AsyncMock(return_value=12)

        result = await client.context_update_current("updated context")

        client.context_write.assert_awaited_once_with("L0/current", "updated context")
        assert result == 12


class TestContextDecisions:
    @pytest.mark.asyncio
    async def test_reads_l1_decisions(self):
        client = _make_client()
        client.context_read = AsyncMock(return_value="# Decisions\n- Use Python\n")

        result = await client.context_decisions()

        client.context_read.assert_awaited_once_with("L1/decisions.md")
        assert "Use Python" in result


class TestContextAddDecision:
    @pytest.mark.asyncio
    async def test_appends_decision(self):
        client = _make_client()
        client.context_read = AsyncMock(return_value="# Decisions\n- Use Python\n")
        client.context_write = AsyncMock(return_value=100)

        result = await client.context_add_decision("Use async")

        expected_content = "# Decisions\n- Use Python\n- Use async\n"
        client.context_write.assert_awaited_once_with("L1/decisions.md", expected_content)
        assert result == 100


class TestContextRecentOps:
    @pytest.mark.asyncio
    async def test_parses_json(self):
        client = _make_client()
        ops = [{"op": "write", "path": "/foo"}, {"op": "read", "path": "/bar"}]
        client.context_read = AsyncMock(return_value=json.dumps(ops))

        result = await client.context_recent_ops()

        assert result == ops


class TestContextSearch:
    @pytest.mark.asyncio
    async def test_search_root(self):
        client = _make_client()
        client.grep = AsyncMock(return_value=["line with pattern"])

        result = await client.context_search("pattern")

        client.grep.assert_awaited_once_with("/context", "pattern", recursive=True)
        assert result == ["line with pattern"]

    @pytest.mark.asyncio
    async def test_search_layer(self):
        client = _make_client()
        client.grep = AsyncMock(return_value=["found in L2"])

        result = await client.context_search("query", layer="L2")

        client.grep.assert_awaited_once_with("/context/L2", "query", recursive=True)
        assert result == ["found in L2"]


class TestContextMeta:
    @pytest.mark.asyncio
    async def test_parses_meta_json(self):
        client = _make_client()
        meta = {"version": 1, "created": "2025-01-01"}
        client.context_read = AsyncMock(return_value=json.dumps(meta))

        result = await client.context_meta()

        assert result == meta


class TestContextKnowledge:
    @pytest.mark.asyncio
    async def test_reads_l2_file(self):
        client = _make_client()
        client.context_read = AsyncMock(return_value="knowledge content")

        result = await client.context_knowledge("architecture.md")

        client.context_read.assert_awaited_once_with("L2/architecture.md")
        assert result == "knowledge content"


class TestContextAddKnowledge:
    @pytest.mark.asyncio
    async def test_writes_l2_file(self):
        client = _make_client()
        client.context_write = AsyncMock(return_value=50)

        result = await client.context_add_knowledge("tips.md", "be helpful")

        client.context_write.assert_awaited_once_with("L2/tips.md", "be helpful")
        assert result == 50


# ===========================================================================
# Skill API tests
# ===========================================================================


class TestSkillDiscover:
    @pytest.mark.asyncio
    async def test_returns_dir_names(self):
        client = _make_client()
        entries = [
            _file_info("summarize", is_dir=True),
            _file_info("translate", is_dir=True),
            _file_info("README.md", is_dir=False),
        ]
        client.ls = AsyncMock(return_value=entries)

        result = await client.skill_discover()

        assert result == ["summarize", "translate"]

    @pytest.mark.asyncio
    async def test_empty_listing(self):
        client = _make_client()
        client.ls = AsyncMock(return_value=[])

        result = await client.skill_discover()

        assert result == []


class TestSkillRead:
    @pytest.mark.asyncio
    async def test_reads_skill_md(self):
        client = _make_client()
        client.cat = AsyncMock(return_value=b"# Summarize\nA skill.")

        result = await client.skill_read("summarize")

        client.cat.assert_awaited_once_with("/skills/summarize/SKILL.md")
        assert result == "# Summarize\nA skill."


class TestSkillExecute:
    @pytest.mark.asyncio
    async def test_writes_input_reads_output(self):
        client = _make_client()
        client.write = AsyncMock(return_value=10)
        client.cat = AsyncMock(return_value=b"execution result")

        result = await client.skill_execute("summarize", "some input")

        client.write.assert_awaited_once_with("/skills/summarize/input", "some input")
        client.cat.assert_awaited_once_with("/skills/summarize/output")
        assert result == "execution result"


class TestSkillRegister:
    @pytest.mark.asyncio
    async def test_creates_dir_and_writes_md(self):
        client = _make_client()
        client.mkdir = AsyncMock(return_value=True)
        client.write = AsyncMock(return_value=100)

        result = await client.skill_register("my-skill", "# My Skill\nDescription")

        client.mkdir.assert_awaited_once_with("/skills/my-skill")
        client.write.assert_awaited_once_with(
            "/skills/my-skill/SKILL.md", "# My Skill\nDescription"
        )
        assert result is True


class TestSkillMatch:
    @pytest.mark.asyncio
    async def test_matches_trigger(self):
        client = _make_client()
        client.ls = AsyncMock(
            return_value=[_file_info("summarize", is_dir=True)]
        )
        skill_md = """---
triggers:
  - summarize
  - tldr
---
# Summarize skill
"""
        client.cat = AsyncMock(return_value=skill_md.encode("utf-8"))

        result = await client.skill_match("please summarize this text")

        assert result == "summarize"

    @pytest.mark.asyncio
    async def test_no_match(self):
        client = _make_client()
        client.ls = AsyncMock(
            return_value=[_file_info("summarize", is_dir=True)]
        )
        skill_md = """---
triggers:
  - summarize
---
# Summarize skill
"""
        client.cat = AsyncMock(return_value=skill_md.encode("utf-8"))

        result = await client.skill_match("translate this to French")

        assert result is None

    @pytest.mark.asyncio
    async def test_no_skills(self):
        client = _make_client()
        client.ls = AsyncMock(return_value=[])

        result = await client.skill_match("anything")

        assert result is None

    @pytest.mark.asyncio
    async def test_error_reading_skill_is_skipped(self):
        client = _make_client()
        client.ls = AsyncMock(
            return_value=[_file_info("broken", is_dir=True)]
        )
        client.cat = AsyncMock(side_effect=Exception("read error"))

        result = await client.skill_match("anything")

        assert result is None


class TestSkillRemove:
    @pytest.mark.asyncio
    async def test_removes_recursively(self):
        client = _make_client()
        client.rm = AsyncMock(return_value=True)

        result = await client.skill_remove("old-skill")

        client.rm.assert_awaited_once_with("/skills/old-skill", recursive=True)
        assert result is True


# ===========================================================================
# Streaming API tests
# ===========================================================================


class TestStreamRead:
    """Tests for stream_read using the /api/v1/fs/stream endpoint."""

    @pytest.mark.asyncio
    async def test_stream_read_yields_chunks(self):
        """Verify stream_read uses httpx streaming and yields byte chunks."""
        client = _make_client()

        # Mock httpx AsyncClient.stream context manager
        mock_response = MagicMock()
        mock_response.status_code = 200
        # Simulate httpx streaming by returning a sample of bytes
        async def mock_aiter_bytes(chunk_size=65536):
            data = b"streamed content here"
            for i in range(0, len(data), chunk_size):
                yield data[i : i + chunk_size]

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)

        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        chunks = []
        async for chunk in client.stream_read("/large/file.bin"):
            chunks.append(chunk)

        # Verify the correct endpoint was called
        client._client.stream.assert_called_once()
        call_kwargs = client._client.stream.call_args
        assert call_kwargs[0][0] == "POST"
        # httpx stores resolved URL internally; verify via json params
        assert call_kwargs[1]["json"] == {
            "op": "read",
            "path": "/large/file.bin",
            "offset": 0,
            "size": 0,
        }

        # Verify chunks were yielded
        combined = b"".join(chunks)
        assert combined == b"streamed content here"

    @pytest.mark.asyncio
    async def test_stream_read_handles_offset_and_size(self):
        """Verify offset and size parameters are passed correctly."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200

        async def mock_aiter_bytes(chunk_size=65536):
            yield b"partial"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_stream_ctx = AsyncMock()
        mock_stream_ctx.__aenter__ = AsyncMock(return_value=mock_response)
        mock_stream_ctx.__aexit__ = AsyncMock(return_value=None)
        client._client.stream = MagicMock(return_value=mock_stream_ctx)

        chunks = []
        async for chunk in client.stream_read("/path/to/file", offset=1024, size=4096):
            chunks.append(chunk)

        call_kwargs = client._client.stream.call_args
        assert call_kwargs[1]["json"] == {
            "op": "read",
            "path": "/path/to/file",
            "offset": 1024,
            "size": 4096,
        }


class TestStreamWrite:
    """Tests for stream_write using the /api/v1/fs/stream endpoint."""

    @pytest.mark.asyncio
    async def test_stream_write_sends_raw_bytes(self):
        """Verify stream_write sends raw bytes to the streaming endpoint."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json = MagicMock(return_value={"bytes_written": 13})

        client._client.post = AsyncMock(return_value=mock_response)

        result = await client.stream_write("/dest/file.bin", b"hello streamed")

        call_kwargs = client._client.post.call_args
        # httpx post() takes path as first positional arg (method is implicit)
        assert call_kwargs[0][0] == "/api/v1/fs/stream"
        assert call_kwargs[1]["content"] == b"hello streamed"
        assert call_kwargs[1]["params"] == {
            "op": "write",
            "path": "/dest/file.bin",
            "offset": -1,
        }
        assert result == 13

    @pytest.mark.asyncio
    async def test_stream_write_with_string_content(self):
        """Verify stream_write encodes string content to UTF-8."""
        client = _make_client()

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json = MagicMock(return_value={"bytes_written": 5})

        client._client.post = AsyncMock(return_value=mock_response)

        result = await client.stream_write("/dest/text.txt", "hello")

        call_kwargs = client._client.post.call_args
        assert call_kwargs[1]["content"] == b"hello"
        assert result == 5
