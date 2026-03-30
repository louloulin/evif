"""Tests for EVIF general-purpose SDK (filesystem, plugins, handles, unified client)."""

import pytest
from unittest.mock import AsyncMock, patch, MagicMock

from evif_mem import (
    EvifClient,
    EvifConfig,
    EvifMemoryClient,
    FileInfo,
    PluginInfo,
    MountInfo,
    HandleInfo,
)
from evif_mem.filesystem import FilesystemOps
from evif_mem.plugins import PluginOps
from evif_mem.handles import HandleOps


@pytest.fixture
def config():
    return EvifConfig(api_url="http://localhost:8081", api_key="test-key")


@pytest.fixture
def mock_response():
    resp = MagicMock()
    resp.raise_for_status = MagicMock()
    resp.json = MagicMock()
    return resp


@pytest.fixture
def client(config):
    with patch("httpx.AsyncClient") as mock_cls:
        mock_http = AsyncMock()
        mock_cls.return_value = mock_http
        c = EvifClient(config)
        c._http = mock_http
        return c


# ── FilesystemOps ────────────────────────────────────────────────


class TestFilesystemOps:
    @pytest.mark.asyncio
    async def test_read_text(self, client, mock_response):
        mock_response.json.return_value = {"data": {"content": "hello world"}}
        client._http.request = AsyncMock(return_value=mock_response)
        text = await client.fs.read_text("/memfs/test.txt")
        assert text == "hello world"

    @pytest.mark.asyncio
    async def test_write_text(self, client, mock_response):
        mock_response.json.return_value = {"data": {"bytes_written": 5}}
        client._http.request = AsyncMock(return_value=mock_response)
        n = await client.fs.write_text("/memfs/test.txt", "hello")
        assert n == 5

    @pytest.mark.asyncio
    async def test_ls(self, client, mock_response):
        mock_response.json.return_value = {
            "data": [
                {"name": "file1.txt", "size": 100, "is_dir": False},
                {"name": "subdir", "size": 0, "is_dir": True},
            ]
        }
        client._http.request = AsyncMock(return_value=mock_response)
        entries = await client.fs.ls("/memfs")
        assert len(entries) == 2
        assert isinstance(entries[0], FileInfo)
        assert entries[0].name == "file1.txt"
        assert not entries[0].is_dir
        assert entries[1].is_dir

    @pytest.mark.asyncio
    async def test_mkdir(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.fs.mkdir("/memfs/newdir", parents=True)
        assert result is True

    @pytest.mark.asyncio
    async def test_rm(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.fs.rm("/memfs/file.txt")
        assert result is True

    @pytest.mark.asyncio
    async def test_stat(self, client, mock_response):
        mock_response.json.return_value = {
            "data": {"name": "file.txt", "size": 42, "mode": 420, "is_dir": False}
        }
        client._http.request = AsyncMock(return_value=mock_response)
        info = await client.fs.stat("/memfs/file.txt")
        assert isinstance(info, FileInfo)
        assert info.name == "file.txt"
        assert info.size == 42

    @pytest.mark.asyncio
    async def test_mv(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.fs.mv("/memfs/a.txt", "/memfs/b.txt")
        assert result is True

    @pytest.mark.asyncio
    async def test_cp(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.fs.cp("/memfs/a.txt", "/memfs/b.txt")
        assert result is True

    @pytest.mark.asyncio
    async def test_touch(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.fs.touch("/memfs/new.txt")
        assert result is True

    @pytest.mark.asyncio
    async def test_grep(self, client, mock_response):
        mock_response.json.return_value = {"data": [{"line": 1, "text": "match"}]}
        client._http.request = AsyncMock(return_value=mock_response)
        results = await client.fs.grep("/memfs", "pattern", recursive=True)
        assert len(results) == 1


# ── PluginOps ────────────────────────────────────────────────────


class TestPluginOps:
    @pytest.mark.asyncio
    async def test_list_plugins(self, client, mock_response):
        mock_response.json.return_value = {
            "data": [
                {"name": "memfs", "mount_path": "/memfs"},
                {"name": "localfs", "mount_path": "/local"},
            ]
        }
        client._http.request = AsyncMock(return_value=mock_response)
        plugins = await client.plugins.list_plugins()
        assert len(plugins) == 2
        assert isinstance(plugins[0], PluginInfo)
        assert plugins[0].name == "memfs"

    @pytest.mark.asyncio
    async def test_list_mounts(self, client, mock_response):
        mock_response.json.return_value = {
            "data": [
                {"path": "/memfs", "plugin_type": "memfs"},
                {"path": "/local", "plugin_type": "localfs", "instance_name": "data"},
            ]
        }
        client._http.request = AsyncMock(return_value=mock_response)
        mounts = await client.plugins.list_mounts()
        assert len(mounts) == 2
        assert isinstance(mounts[0], MountInfo)
        assert mounts[0].path == "/memfs"
        assert mounts[1].instance_name == "data"

    @pytest.mark.asyncio
    async def test_mount(self, client, mock_response):
        mock_response.json.return_value = {
            "data": {"path": "/s3/aws", "plugin_type": "s3fs", "instance_name": "aws"}
        }
        client._http.request = AsyncMock(return_value=mock_response)
        info = await client.plugins.mount(
            "s3fs",
            "/s3/aws",
            config={"region": "us-west-1"},
            instance_name="aws",
        )
        assert isinstance(info, MountInfo)
        assert info.path == "/s3/aws"
        assert info.plugin_type == "s3fs"

    @pytest.mark.asyncio
    async def test_unmount(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.plugins.unmount("/s3/aws")
        assert result is True

    @pytest.mark.asyncio
    async def test_get_readme(self, client, mock_response):
        mock_response.json.return_value = {
            "data": {"readme": "# MemFS\n\nMemory filesystem."}
        }
        client._http.request = AsyncMock(return_value=mock_response)
        readme = await client.plugins.get_readme("memfs")
        assert "MemFS" in readme


# ── HandleOps ────────────────────────────────────────────────────


class TestHandleOps:
    @pytest.mark.asyncio
    async def test_open(self, client, mock_response):
        mock_response.json.return_value = {
            "data": {"id": "h-1", "path": "/memfs/file.txt", "mode": "read", "offset": 0}
        }
        client._http.request = AsyncMock(return_value=mock_response)
        handle = await client.handles.open("/memfs/file.txt", mode="read")
        assert isinstance(handle, HandleInfo)
        assert handle.id == "h-1"
        assert handle.path == "/memfs/file.txt"

    @pytest.mark.asyncio
    async def test_read(self, client, mock_response):
        mock_response.json.return_value = {"data": {"content": "hello"}}
        client._http.request = AsyncMock(return_value=mock_response)
        data = await client.handles.read("h-1", size=5)
        assert data == b"hello"

    @pytest.mark.asyncio
    async def test_write(self, client, mock_response):
        mock_response.json.return_value = {"data": {"bytes_written": 5}}
        client._http.request = AsyncMock(return_value=mock_response)
        n = await client.handles.write("h-1", b"hello")
        assert n == 5

    @pytest.mark.asyncio
    async def test_seek(self, client, mock_response):
        mock_response.json.return_value = {"data": {"offset": 42}}
        client._http.request = AsyncMock(return_value=mock_response)
        pos = await client.handles.seek("h-1", 42)
        assert pos == 42

    @pytest.mark.asyncio
    async def test_close(self, client, mock_response):
        mock_response.json.return_value = {"data": {}}
        client._http.request = AsyncMock(return_value=mock_response)
        result = await client.handles.close("h-1")
        assert result is True

    @pytest.mark.asyncio
    async def test_list_handles(self, client, mock_response):
        mock_response.json.return_value = {
            "data": [
                {"id": "h-1", "path": "/memfs/a.txt", "mode": "read", "offset": 0, "ttl": 300},
            ]
        }
        client._http.request = AsyncMock(return_value=mock_response)
        handles = await client.handles.list_handles()
        assert len(handles) == 1
        assert isinstance(handles[0], HandleInfo)

    @pytest.mark.asyncio
    async def test_renew(self, client, mock_response):
        mock_response.json.return_value = {
            "data": {"id": "h-1", "path": "/memfs/a.txt", "mode": "read", "offset": 0, "ttl": 600}
        }
        client._http.request = AsyncMock(return_value=mock_response)
        info = await client.handles.renew("h-1")
        assert info.ttl == 600


# ── EvifClient unified ──────────────────────────────────────────


class TestEvifClient:
    def test_sub_clients_exist(self, client):
        assert isinstance(client.fs, FilesystemOps)
        assert isinstance(client.plugins, PluginOps)
        assert isinstance(client.handles, HandleOps)
        assert isinstance(client.memory, EvifMemoryClient)

    @pytest.mark.asyncio
    async def test_health(self, client, mock_response):
        mock_response.json.return_value = {"data": {"status": "ok", "version": "0.1.0"}}
        client._http.request = AsyncMock(return_value=mock_response)
        health = await client.health()
        assert health["status"] == "ok"

    @pytest.mark.asyncio
    async def test_context_manager(self, config):
        with patch("httpx.AsyncClient") as mock_cls:
            mock_http = AsyncMock()
            mock_cls.return_value = mock_http
            mock_http.aclose = AsyncMock()

            async with EvifClient(config) as c:
                assert c.fs is not None

            mock_http.aclose.assert_called_once()


# ── Model repr ───────────────────────────────────────────────────


class TestModelRepr:
    def test_file_info_repr(self):
        fi = FileInfo(name="test.txt", is_dir=False, size=100)
        assert "test.txt" in repr(fi)
        assert "file" in repr(fi)

    def test_plugin_info_repr(self):
        pi = PluginInfo(name="memfs", mount_path="/memfs")
        assert "memfs" in repr(pi)

    def test_mount_info_repr(self):
        mi = MountInfo(path="/memfs", plugin_type="memfs")
        assert "/memfs" in repr(mi)

    def test_handle_info_repr(self):
        hi = HandleInfo(id="h-1", path="/test.txt")
        assert "h-1" in repr(hi)
