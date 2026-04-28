"""EVIF client models."""

from datetime import datetime
from typing import Optional
from pydantic import BaseModel, Field


class FileInfo(BaseModel):
    """File metadata information."""

    name: str
    path: str
    size: int
    mode: int
    mtime: float
    is_dir: bool
    is_file: bool = True

    @classmethod
    def from_dict(cls, data: dict) -> "FileInfo":
        """Create FileInfo from API response.

        Handles both list (ls) and stat response formats.
        ls: {name, path, size, is_dir, modified, created}
        stat: {path, size, is_dir, modified, created}
        """
        import datetime
        name = data.get("name", "")
        if not name:
            name = data.get("path", "").rstrip("/").split("/")[-1]
        mtime_val = data.get("mtime", 0)
        if not mtime_val and data.get("modified"):
            try:
                dt = datetime.datetime.fromisoformat(data["modified"])
                mtime_val = dt.timestamp()
            except (ValueError, TypeError):
                mtime_val = 0
        return cls(
            name=name,
            path=data.get("path", ""),
            size=data.get("size", 0),
            mode=data.get("mode", 0o644),
            mtime=mtime_val,
            is_dir=data.get("is_dir", False),
            is_file=not data.get("is_dir", True),
        )


class MountInfo(BaseModel):
    """Plugin mount information."""

    path: str
    plugin: str
    options: dict = Field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict) -> "MountInfo":
        """Create MountInfo from API response."""
        return cls(
            path=data.get("path", ""),
            plugin=data.get("plugin", ""),
            options=data.get("options", {}),
        )


class HealthStatus(BaseModel):
    """Server health status."""

    status: str
    version: str
    uptime: float
    plugins_count: int

    @classmethod
    def from_dict(cls, data: dict) -> "HealthStatus":
        """Create HealthStatus from API response."""
        return cls(
            status=data.get("status", "unknown"),
            version=data.get("version", "0.0.0"),
            uptime=data.get("uptime", 0.0),
            plugins_count=data.get("plugins_count", 0),
        )


class HandleInfo(BaseModel):
    """File handle information."""

    id: int
    path: str
    flags: int
    offset: int
    expires_at: float

    @classmethod
    def from_dict(cls, data: dict) -> "HandleInfo":
        """Create HandleInfo from API response."""
        return cls(
            id=data.get("id", 0),
            path=data.get("path", ""),
            flags=data.get("flags", 0),
            offset=data.get("offset", 0),
            expires_at=data.get("expires_at", 0),
        )
