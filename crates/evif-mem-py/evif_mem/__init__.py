"""EVIF Memory Python SDK

A Python SDK for the EVIF Memory Platform.
AI-native memory storage with vector search, categorization, and self-evolution.
"""

__version__ = "0.1.0"

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

__all__ = [
    "EvifMemoryClient",
    "MemoryConfig",
    "Memory",
    "MemoryCreate",
    "MemorySearchResult",
    "Category",
    "MemoryQuery",
    "MemoryQueryType",
    "MemoryQueryResult",
]
