"""EVIF Python SDK - Everything is a File System

A Python client for interacting with EVIF filesystem and plugins.
"""

__version__ = "0.1.0"
__author__ = "EVIF Development Team"

from evif.client import EvifClient
from evif.file_handle import FileHandle
from evif.exceptions import (
    EvifError,
    ClientError,
    AuthenticationError,
    FileNotFoundError,
    PermissionError,
    TimeoutError,
)

__all__ = [
    "EvifClient",
    "FileHandle",
    "EvifError",
    "ClientError",
    "AuthenticationError",
    "FileNotFoundError",
    "PermissionError",
    "TimeoutError",
]
