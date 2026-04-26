"""EVIF Python SDK - Everything is a File System

A Python client for interacting with EVIF filesystem and plugins.
"""

__version__ = "0.1.0"
__author__ = "EVIF Development Team"

from evif.client import EvifClient
from evif.file_handle import FileHandle
from evif.context import ContextApi
from evif.skill import SkillApi
from evif.memory import MemoryApi
from evif.queue import QueueApi
from evif.sync import Client, SyncEvifClient
from evif.exceptions import (
    EvifError,
    ClientError,
    AuthenticationError,
    FileNotFoundError,
    PermissionError,
    TimeoutError,
)

__all__ = [
    # Main entry point
    "Client",
    # Classes
    "EvifClient",
    "SyncEvifClient",
    "FileHandle",
    "ContextApi",
    "SkillApi",
    "MemoryApi",
    "QueueApi",
    # Exceptions
    "EvifError",
    "ClientError",
    "AuthenticationError",
    "FileNotFoundError",
    "PermissionError",
    "TimeoutError",
]
