"""EVIF Memory Client - Main client for interacting with the EVIF Memory API."""

import asyncio
from typing import List, Optional, Dict, Any

import httpx

from .config import MemoryConfig
from .models import (
    Memory,
    MemoryCreate,
    MemorySearchResult,
    Category,
    GraphQuery,
    GraphQueryType,
    GraphResult,
    GraphNode,
    GraphEdge,
)


class EvifMemoryClient:
    """Client for the EVIF Memory API.

    Provides methods for creating, retrieving, and searching memories.

    Args:
        config: Client configuration

    Example:
        >>> config = MemoryConfig(api_url="http://localhost:8080")
        >>> client = EvifMemoryClient(config)
        >>> memory = await client.create_memory("Hello world", tags=["greeting"])
    """

    def __init__(self, config: MemoryConfig):
        """Initialize the client with configuration."""
        self._config = config
        self._client = httpx.AsyncClient(
            base_url=config.api_url,
            timeout=config.timeout,
            headers=self._build_headers(),
        )

    def _build_headers(self) -> Dict[str, str]:
        """Build request headers."""
        headers = {
            "Content-Type": "application/json",
            "User-Agent": "evif-mem-python/0.1.0",
        }
        if self._config.api_key:
            headers["Authorization"] = f"Bearer {self._config.api_key}"
        return headers

    async def _request(
        self,
        method: str,
        path: str,
        **kwargs: Any,
    ) -> Any:
        """Make an HTTP request with retry logic."""
        retries = 0
        last_error = None

        while retries <= self._config.max_retries:
            try:
                response = await self._client.request(method, path, **kwargs)
                response.raise_for_status()
                return response.json()
            except httpx.HTTPError as e:
                last_error = e
                retries += 1
                if retries <= self._config.max_retries:
                    await asyncio.sleep(0.5 * retries)

        raise RuntimeError(f"Request failed after {self._config.max_retries} retries: {last_error}")

    async def create_memory(
        self,
        content: str,
        memory_type: str = "knowledge",
        tags: Optional[List[str]] = None,
        modality: str = "text",
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Memory:
        """Create a new memory.

        Args:
            content: Memory content
            memory_type: Type of memory (profile, event, knowledge, behavior, skill, tool)
            tags: List of tags
            modality: Input modality (text, conversation, document, image, video, audio)
            metadata: Additional metadata

        Returns:
            Created memory object

        Example:
            >>> memory = await client.create_memory(
            ...     "User prefers dark mode",
            ...     memory_type="preference",
            ...     tags=["ui", "preferences"]
            ... )
        """
        data = {
            "content": content,
            "memory_type": memory_type,
            "tags": tags or [],
            "modality": modality,
            "metadata": metadata or {},
        }
        result = await self._request("POST", "/api/v1/memories", json=data)
        return Memory(**result["data"])

    async def get_memory(self, memory_id: str) -> Memory:
        """Get a specific memory by ID.

        Args:
            memory_id: Memory ID

        Returns:
            Memory object

        Example:
            >>> memory = await client.get_memory("mem-123")
        """
        result = await self._request("GET", f"/api/v1/memories/{memory_id}")
        return Memory(**result["data"])

    async def list_memories(
        self,
        limit: int = 100,
        offset: int = 0,
    ) -> List[Memory]:
        """List all memories.

        Args:
            limit: Maximum number of results
            offset: Offset for pagination

        Returns:
            List of memories

        Example:
            >>> memories = await client.list_memories(limit=10)
        """
        params = {"limit": limit, "offset": offset}
        result = await self._request("GET", "/api/v1/memories", params=params)
        return [Memory(**item) for item in result.get("data", [])]

    async def search_memories(
        self,
        query: str,
        k: int = 10,
        threshold: float = 0.0,
        mode: str = "vector",
    ) -> List[MemorySearchResult]:
        """Search memories by semantic similarity.

        Args:
            query: Search query
            k: Number of results to return
            threshold: Minimum similarity threshold (0-1)
            mode: Search mode (vector, hybrid, rag)

        Returns:
            List of search results with scores

        Example:
            >>> results = await client.search_memories("user preferences", k=5)
            >>> for result in results:
            ...     print(f"{result.memory.content} (score: {result.score})")
        """
        data = {
            "query": query,
            "k": k,
            "threshold": threshold,
            "mode": mode,
        }
        result = await self._request("POST", "/api/v1/memories/search", json=data)
        return [
            MemorySearchResult(memory=Memory(**item["memory"]), score=item["score"])
            for item in result.get("data", [])
        ]

    async def delete_memory(self, memory_id: str) -> bool:
        """Delete a memory.

        Args:
            memory_id: Memory ID to delete

        Returns:
            True if successful

        Example:
            >>> await client.delete_memory("mem-123")
        """
        await self._request("DELETE", f"/api/v1/memories/{memory_id}")
        return True

    async def list_categories(self) -> List[Category]:
        """List all categories.

        Returns:
            List of categories

        Example:
            >>> categories = await client.list_categories()
        """
        result = await self._request("GET", "/api/v1/categories")
        return [Category(**item) for item in result.get("data", [])]

    async def get_category(self, category_id: str) -> Category:
        """Get a specific category.

        Args:
            category_id: Category ID

        Returns:
            Category object

        Example:
            >>> category = await client.get_category("cat-123")
        """
        result = await self._request("GET", f"/api/v1/categories/{category_id}")
        return Category(**result["data"])

    async def get_category_memories(
        self,
        category_id: str,
        limit: int = 100,
    ) -> List[Memory]:
        """Get memories in a category.

        Args:
            category_id: Category ID
            limit: Maximum number of results

        Returns:
            List of memories in the category

        Example:
            >>> memories = await client.get_category_memories("cat-123")
        """
        params = {"limit": limit}
        result = await self._request(
            "GET",
            f"/api/v1/categories/{category_id}/memories",
            params=params
        )
        return [Memory(**item) for item in result.get("data", [])]

    async def query_graph(
        self,
        query: str,
        query_type: str = "causal_chain",
        node_id: Optional[str] = None,
        max_depth: int = 3,
        limit: int = 10,
    ) -> GraphResult:
        """Query the knowledge graph.

        Args:
            query: Query string
            query_type: Type of query (causal_chain, timeline, temporal_bfs, temporal_path)
            node_id: Starting node ID
            max_depth: Maximum traversal depth
            limit: Maximum results

        Returns:
            Graph query result

        Example:
            >>> result = await client.query_graph(
            ...     "related events",
            ...     query_type="causal_chain"
            ... )
            >>> for node in result.nodes:
            ...     print(node.label)
        """
        data = {
            "query": query,
            "query_type": query_type,
            "node_id": node_id,
            "max_depth": max_depth,
            "limit": limit,
        }
        result = await self._request("POST", "/api/v1/graph/query", json=data)
        data = result.get("data", {})
        return GraphResult(
            nodes=[GraphNode(**node) for node in data.get("nodes", [])],
            edges=[GraphEdge(**edge) for edge in data.get("edges", [])],
            metadata=data.get("metadata", {}),
        )

    async def close(self) -> None:
        """Close the client and release resources."""
        await self._client.aclose()

    async def __aenter__(self) -> "EvifMemoryClient":
        """Async context manager entry."""
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Async context manager exit."""
        await self.close()
