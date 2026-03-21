"""Data models for EVIF Memory."""

from datetime import datetime
from enum import Enum
from typing import Dict, List, Optional, Any
from pydantic import BaseModel, Field


class MemoryType(str, Enum):
    """Types of memory."""

    PROFILE = "profile"
    EVENT = "event"
    KNOWLEDGE = "knowledge"
    BEHAVIOR = "behavior"
    SKILL = "skill"
    TOOL = "tool"
    CONVERSATION = "conversation"
    DOCUMENT = "document"


class Modality(str, Enum):
    """Input modality types."""

    TEXT = "text"
    CONVERSATION = "conversation"
    DOCUMENT = "document"
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"


class MemoryQueryType(str, Enum):
    """Types of memory timeline/relationship queries."""

    CAUSAL_CHAIN = "causal_chain"
    TIMELINE = "timeline"
    TEMPORAL_BFS = "temporal_bfs"
    TEMPORAL_PATH = "temporal_path"


class MemoryCreate(BaseModel):
    """Request model for creating a memory."""

    content: str = Field(..., description="Memory content")
    memory_type: MemoryType = Field(default=MemoryType.KNOWLEDGE, description="Type of memory")
    tags: List[str] = Field(default_factory=list, description="Memory tags")
    modality: Modality = Field(default=Modality.TEXT, description="Input modality")
    references: List[str] = Field(default_factory=list, description="Related memory IDs")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Additional metadata")


class Memory(BaseModel):
    """Memory item model."""

    id: str = Field(..., description="Memory ID")
    content: str = Field(..., description="Memory content")
    summary: str = Field(..., description="Memory summary")
    memory_type: MemoryType = Field(..., description="Type of memory")
    tags: List[str] = Field(default_factory=list, description="Memory tags")
    embedding: Optional[List[float]] = Field(default=None, description="Vector embedding")
    reinforcement_count: int = Field(default=0, description="Times memory was reinforced")
    last_reinforced_at: Optional[datetime] = Field(default=None, description="Last reinforcement time")
    created_at: datetime = Field(..., description="Creation timestamp")
    updated_at: datetime = Field(..., description="Last update timestamp")
    references: List[str] = Field(default_factory=list, description="Related memory IDs")
    user_id: Optional[str] = Field(default=None, description="User ID")
    tenant_id: Optional[str] = Field(default=None, description="Tenant ID")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Additional metadata")


class MemorySearchResult(BaseModel):
    """Memory search result with score."""

    memory: Memory = Field(..., description="The memory item")
    score: float = Field(..., description="Relevance score (0-1)")


class Category(BaseModel):
    """Memory category model."""

    id: str = Field(..., description="Category ID")
    name: str = Field(..., description="Category name")
    description: str = Field(..., description="Category description")
    summary: str = Field(..., description="Category summary")
    item_count: int = Field(default=0, description="Number of items in category")
    embedding: Optional[List[float]] = Field(default=None, description="Category embedding")
    created_at: datetime = Field(..., description="Creation timestamp")
    updated_at: datetime = Field(..., description="Last update timestamp")


class MemoryQuery(BaseModel):
    """Memory query model."""

    query_type: MemoryQueryType = Field(..., description="Type of memory query")
    start_node: Optional[str] = Field(default=None, description="Starting memory ID")
    end_node: Optional[str] = Field(default=None, description="Ending memory ID")
    max_depth: Optional[int] = Field(default=3, description="Maximum traversal depth")
    event_type: Optional[str] = Field(default=None, description="Event type filter")
    category: Optional[str] = Field(default=None, description="Category filter")
    start_time: Optional[str] = Field(default=None, description="RFC3339 start time")
    end_time: Optional[str] = Field(default=None, description="RFC3339 end time")


class MemoryQueryNode(BaseModel):
    """Memory query node model."""

    id: str = Field(..., description="Node ID")
    node_type: str = Field(..., description="Node type")
    label: str = Field(..., description="Node label")
    timestamp: Optional[str] = Field(default=None, description="Node timestamp")


class TimelineEvent(BaseModel):
    """Timeline event model."""

    node_id: str = Field(..., description="Memory ID")
    timestamp: str = Field(..., description="Event timestamp")
    event_type: str = Field(..., description="Event type")


class MemoryQueryPath(BaseModel):
    """Memory query path model."""

    nodes: List[str] = Field(default_factory=list, description="Path nodes")
    edges: List[str] = Field(default_factory=list, description="Path edge labels")
    narrative: str = Field(default="", description="Path narrative")


class MemoryQueryResult(BaseModel):
    """Memory query result."""

    query_type: str = Field(..., description="Executed query type")
    nodes: List[MemoryQueryNode] = Field(default_factory=list, description="Result nodes")
    timeline: List[TimelineEvent] = Field(default_factory=list, description="Timeline events")
    paths: List[MemoryQueryPath] = Field(default_factory=list, description="Result paths")
    total: int = Field(default=0, description="Total result count")


# Backward-compatible aliases. Prefer the MemoryQuery* names going forward.
GraphQueryType = MemoryQueryType
GraphQuery = MemoryQuery
GraphNode = MemoryQueryNode
GraphResult = MemoryQueryResult
