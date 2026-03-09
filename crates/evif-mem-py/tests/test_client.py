"""Tests for EVIF Memory Python SDK."""

import pytest
from unittest.mock import AsyncMock, patch, MagicMock
from datetime import datetime

from evif_mem import EvifMemoryClient, MemoryConfig
from evif_mem.models import (
    Memory,
    Category,
    MemorySearchResult,
    GraphResult,
    GraphNode,
    GraphEdge,
)


@pytest.fixture
def config():
    """Create test configuration."""
    return MemoryConfig(
        api_url="http://localhost:8080",
        api_key="test-api-key",
    )


@pytest.fixture
def mock_response():
    """Create a mock HTTP response."""
    response = MagicMock()
    response.raise_for_status = MagicMock()
    response.json = MagicMock()
    return response


@pytest.fixture
def client(config):
    """Create client with mocked HTTP client."""
    with patch("httpx.AsyncClient") as mock_client_class:
        mock_client = AsyncMock()
        mock_client_class.return_value = mock_client

        client = EvifMemoryClient(config)
        client._client = mock_client
        return client


class TestMemoryConfig:
    """Tests for MemoryConfig."""

    def test_default_config(self):
        """Test default configuration values."""
        config = MemoryConfig()
        assert config.api_url == "http://localhost:8080"
        assert config.api_key is None
        assert config.timeout == 30
        assert config.max_retries == 3

    def test_custom_config(self):
        """Test custom configuration values."""
        config = MemoryConfig(
            api_url="https://api.example.com",
            api_key="secret-key",
            timeout=60,
            max_retries=5,
        )
        assert config.api_url == "https://api.example.com"
        assert config.api_key == "secret-key"
        assert config.timeout == 60
        assert config.max_retries == 5


class TestEvifMemoryClient:
    """Tests for EvifMemoryClient."""

    @pytest.mark.asyncio
    async def test_create_memory(self, client, mock_response):
        """Test creating a memory."""
        mock_response.json.return_value = {
            "data": {
                "id": "mem-123",
                "content": "Test memory",
                "summary": "Test summary",
                "memory_type": "knowledge",
                "tags": ["test"],
                "embedding": None,
                "reinforcement_count": 0,
                "last_reinforced_at": None,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "references": [],
                "user_id": None,
                "tenant_id": None,
                "metadata": {},
            }
        }
        client._client.request = AsyncMock(return_value=mock_response)

        memory = await client.create_memory("Test memory", tags=["test"])

        assert memory.id == "mem-123"
        assert memory.content == "Test memory"
        assert memory.memory_type == "knowledge"

    @pytest.mark.asyncio
    async def test_list_memories(self, client, mock_response):
        """Test listing memories."""
        mock_response.json.return_value = {
            "data": [
                {
                    "id": "mem-123",
                    "content": "Test memory",
                    "summary": "Test summary",
                    "memory_type": "knowledge",
                    "tags": ["test"],
                    "embedding": None,
                    "reinforcement_count": 0,
                    "last_reinforced_at": None,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "references": [],
                    "user_id": None,
                    "tenant_id": None,
                    "metadata": {},
                }
            ]
        }
        client._client.request = AsyncMock(return_value=mock_response)

        memories = await client.list_memories()

        assert len(memories) == 1
        assert memories[0].id == "mem-123"

    @pytest.mark.asyncio
    async def test_search_memories(self, client, mock_response):
        """Test searching memories."""
        mock_response.json.return_value = {
            "data": [
                {
                    "memory": {
                        "id": "mem-123",
                        "content": "Test memory",
                        "summary": "Test summary",
                        "memory_type": "knowledge",
                        "tags": ["test"],
                        "embedding": None,
                        "reinforcement_count": 0,
                        "last_reinforced_at": None,
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z",
                        "references": [],
                        "user_id": None,
                        "tenant_id": None,
                        "metadata": {},
                    },
                    "score": 0.95,
                }
            ]
        }
        client._client.request = AsyncMock(return_value=mock_response)

        results = await client.search_memories("test query", k=10)

        assert len(results) == 1
        assert results[0].score == 0.95
        assert results[0].memory.id == "mem-123"

    @pytest.mark.asyncio
    async def test_list_categories(self, client, mock_response):
        """Test listing categories."""
        mock_response.json.return_value = {
            "data": [
                {
                    "id": "cat-123",
                    "name": "Test Category",
                    "description": "Test description",
                    "summary": "Test summary",
                    "item_count": 10,
                    "embedding": None,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                }
            ]
        }
        client._client.request = AsyncMock(return_value=mock_response)

        categories = await client.list_categories()

        assert len(categories) == 1
        assert categories[0].name == "Test Category"

    @pytest.mark.asyncio
    async def test_query_graph(self, client, mock_response):
        """Test querying the knowledge graph."""
        mock_response.json.return_value = {
            "data": {
                "nodes": [
                    {
                        "id": "node-1",
                        "node_type": "memory",
                        "label": "Test Node",
                        "metadata": {},
                    }
                ],
                "edges": [
                    {
                        "id": "edge-1",
                        "source": "node-1",
                        "target": "node-2",
                        "edge_type": "references",
                        "metadata": {},
                    }
                ],
                "metadata": {},
            }
        }
        client._client.request = AsyncMock(return_value=mock_response)

        result = await client.query_graph("test query", query_type="causal_chain")

        assert len(result.nodes) == 1
        assert result.nodes[0].id == "node-1"
        assert len(result.edges) == 1
        assert result.edges[0].edge_type == "references"

    @pytest.mark.asyncio
    async def test_context_manager(self, config):
        """Test async context manager."""
        with patch("httpx.AsyncClient") as mock_client_class:
            mock_client = AsyncMock()
            mock_client_class.return_value = mock_client
            mock_client.aclose = AsyncMock()

            async with EvifMemoryClient(config) as client:
                pass

            mock_client.aclose.assert_called_once()


class TestModels:
    """Tests for data models."""

    def test_memory_model(self):
        """Test Memory model."""
        memory = Memory(
            id="mem-123",
            content="Test content",
            summary="Test summary",
            memory_type="knowledge",
            tags=["test"],
            created_at=datetime.now(),
            updated_at=datetime.now(),
        )
        assert memory.id == "mem-123"
        assert memory.content == "Test content"

    def test_category_model(self):
        """Test Category model."""
        category = Category(
            id="cat-123",
            name="Test Category",
            description="Test description",
            summary="Test summary",
            created_at=datetime.now(),
            updated_at=datetime.now(),
        )
        assert category.id == "cat-123"
        assert category.name == "Test Category"

    def test_memory_search_result(self):
        """Test MemorySearchResult model."""
        memory = Memory(
            id="mem-123",
            content="Test",
            summary="Test",
            memory_type="knowledge",
            tags=[],
            created_at=datetime.now(),
            updated_at=datetime.now(),
        )
        result = MemorySearchResult(memory=memory, score=0.95)
        assert result.score == 0.95
        assert result.memory.id == "mem-123"
