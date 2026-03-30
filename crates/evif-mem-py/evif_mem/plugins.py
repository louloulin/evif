"""Plugin management operations for EVIF SDK."""

from typing import Any, Dict, List, Optional

from .config import MemoryConfig


class PluginInfo:
    """Information about a loaded plugin."""

    def __init__(self, name: str, mount_path: str = "", **kwargs: Any):
        self.name = name
        self.mount_path = mount_path

    def __repr__(self) -> str:
        return f"PluginInfo({self.name!r}, mount={self.mount_path!r})"


class MountInfo:
    """Information about a mount point."""

    def __init__(
        self,
        path: str,
        plugin_type: str = "",
        instance_name: str = "",
        **kwargs: Any,
    ):
        self.path = path
        self.plugin_type = plugin_type
        self.instance_name = instance_name

    def __repr__(self) -> str:
        return f"MountInfo({self.path!r}, plugin={self.plugin_type!r})"


class PluginOps:
    """Plugin management operations against the EVIF REST API.

    Access via ``EvifClient.plugins``.
    """

    def __init__(self, config: MemoryConfig, request_fn: Any):
        self._config = config
        self._request = request_fn

    async def list_plugins(self) -> List[PluginInfo]:
        """List all loaded plugins.

        Returns:
            List of ``PluginInfo`` objects.
        """
        result = await self._request("GET", "/api/v1/plugins")
        items = result.get("data", [])
        if isinstance(items, dict):
            items = items.get("plugins", [])
        return [
            PluginInfo(
                name=item.get("name", ""),
                mount_path=item.get("mount_path", item.get("path", "")),
            )
            for item in items
        ]

    async def list_available(self) -> List[Dict[str, Any]]:
        """List available plugin types.

        Returns:
            List of plugin type descriptors.
        """
        result = await self._request("GET", "/api/v1/plugins/available")
        return result.get("data", [])

    async def get_readme(self, name: str) -> str:
        """Get plugin README.

        Args:
            name: Plugin name.

        Returns:
            README content as string.
        """
        result = await self._request("GET", f"/api/v1/plugins/{name}/readme")
        return result.get("data", {}).get("readme", "")

    async def get_status(self, name: str) -> Dict[str, Any]:
        """Get plugin status.

        Args:
            name: Plugin name.

        Returns:
            Status dictionary.
        """
        result = await self._request("GET", f"/api/v1/plugins/{name}/status")
        return result.get("data", {})

    async def reload(self, name: str) -> bool:
        """Reload a plugin.

        Args:
            name: Plugin name.

        Returns:
            True if successful.
        """
        await self._request("POST", f"/api/v1/plugins/{name}/reload")
        return True

    async def list_mounts(self) -> List[MountInfo]:
        """List all mount points.

        Returns:
            List of ``MountInfo`` objects.
        """
        result = await self._request("GET", "/api/v1/mounts")
        items = result.get("data", [])
        if isinstance(items, dict):
            items = items.get("mounts", [])
        return [
            MountInfo(
                path=item.get("path", ""),
                plugin_type=item.get("plugin_type", item.get("type", "")),
                instance_name=item.get("instance_name", ""),
            )
            for item in items
        ]

    async def mount(
        self,
        plugin_type: str,
        path: str,
        config: Optional[Dict[str, Any]] = None,
        instance_name: Optional[str] = None,
    ) -> MountInfo:
        """Mount a plugin at a given path.

        Args:
            plugin_type: Plugin type (e.g. ``memfs``, ``localfs``, ``s3fs``).
            path: Mount path (e.g. ``/mymem``).
            config: Optional plugin configuration.
            instance_name: Optional instance name for multi-instance support.

        Returns:
            ``MountInfo`` for the new mount.
        """
        body: Dict[str, Any] = {
            "plugin_type": plugin_type,
            "path": path,
        }
        if config:
            body["config"] = config
        if instance_name:
            body["instance_name"] = instance_name
        result = await self._request("POST", "/api/v1/mount", json=body)
        data = result.get("data", result)
        return MountInfo(
            path=data.get("path", path),
            plugin_type=data.get("plugin_type", plugin_type),
            instance_name=data.get("instance_name", instance_name or ""),
        )

    async def unmount(self, path: str) -> bool:
        """Unmount a plugin.

        Args:
            path: Mount path to unmount.

        Returns:
            True if successful.
        """
        await self._request("POST", "/api/v1/unmount", json={"path": path})
        return True
