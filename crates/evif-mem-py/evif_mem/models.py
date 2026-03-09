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


class GraphQueryType(str, Enum):
    """Types of graph queries."""

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


class GraphQuery(BaseModel):
    """Graph query model."""

    query: str = Field(..., description="Query string")
    query_type: GraphQueryType = Field(..., description="Type of graph query")
    node_id: Optional[str] = Field(default=None, description="Starting node ID")
    max_depth: Optional[int] = Field(default=3, description="Maximum traversal depth")
    limit: Optional[int] = Field(default=10, description="Maximum results")


class GraphNode(BaseModel):
    """Graph node model."""

    id: str = Field(..., description="Node ID")
    node_type: str = Field(..., description="Node type")
    label: str = Field(..., description="Node label")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Node metadata")


class GraphEdge(BaseModel):
    """Graph edge model."""

    id: str = Field(..., description="Edge ID")
    source: str = Field(..., description="Source node ID")
    target: str = Field(..., description="Target node ID")
    edge_type: str = Field(..., description="Edge type")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Edge metadata")


class GraphResult(BaseModel):
    """Graph query result."""

    nodes: List[GraphNode] = Field(default_factory=list, description="Result nodes")
    edges: List[GraphEdge] = Field(default_factory=list, description="Result edges")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Query metadata")
