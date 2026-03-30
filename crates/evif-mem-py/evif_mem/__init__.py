"""EVIF Python SDK

A Python SDK for the EVIF platform — Everything Is a File.
Provides filesystem operations, plugin management, stateful handles,
and an AI-native memory platform.

Usage::

    from evif_mem import EvifClient, EvifConfig

    config = EvifConfig(api_url="http://localhost:8080")
    async with EvifClient(config) as client:
        # Filesystem
        await client.fs.write_text("/memfs/hello.txt", "Hello!")
        text = await client.fs.read_text("/memfs/hello.txt")

        # Plugins
        mounts = await client.plugins.list_mounts()

        # Memory
        mem = await client.memory.create_memory("Important note")
"""

__version__ = "0.2.0"

# Unified client (recommended)
from .evif_client import EvifClient, EvifConfig

# Legacy memory-only client (still supported)
from .client import EvifMemoryClient
from .config import MemoryConfig
from .models import (
    Memory,
    MemoryCreate,
    MemorySearchResult,
    Category,
    MemoryQuery,
    MemoryQueryType,
    MemoryQueryResult,
)

# Sub-module types
from .filesystem import FileInfo
from .plugins import PluginInfo, MountInfo
from .handles import HandleInfo

__all__ = [
    # Unified client
    "EvifClient",
    "EvifConfig",
    # Legacy memory client
    "EvifMemoryClient",
    "MemoryConfig",
    # Models
    "Memory",
    "MemoryCreate",
    "MemorySearchResult",
    "Category",
    "MemoryQuery",
    "MemoryQueryType",
    "MemoryQueryResult",
    # Filesystem
    "FileInfo",
    # Plugins
    "PluginInfo",
    "MountInfo",
    # Handles
    "HandleInfo",
]
