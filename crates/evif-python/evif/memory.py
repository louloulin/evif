"""EVIF Memory API - Memory storage and retrieval operations."""

import json
from typing import Optional, Union


class MemoryApi:
    """Mixin providing Memory API operations for EvifClient.

    The memory API stores and retrieves content in the EVIF memory system.
    Memories have a modality (text, code, data, image, audio, video) and
    can be searched by content.
    """

    async def memory_store(
        self,
        content: str,
        modality: str = "text",
        metadata: Optional[dict] = None,
    ) -> dict:
        """Store content in memory.

        Uses the EVIF memory system via REST API or filesystem operations.
        When REST API is available, uses POST /api/v1/memories.
        Otherwise, falls back to filesystem operations.

        Args:
            content: Content to store
            modality: Type of content (text, code, data, image, audio, video)
            metadata: Optional metadata dictionary

        Returns:
            Dict with memory ID and status
        """
        # Try REST API first
        try:
            payload = {
                "content": content,
                "modality": modality,
                "metadata": metadata or {},
            }
            result = await self._request("POST", "/api/v1/memories", json=payload)
            return result
        except Exception:
            pass

        # Fallback: write to memfs via filesystem
        import time

        memory_id = f"mem_{int(time.time() * 1000)}"
        mem_path = f"/memories/{memory_id}"

        # Ensure /memories exists
        try:
            await self.mkdir("/memories")
        except Exception:
            pass

        memory_data = {
            "id": memory_id,
            "content": content,
            "modality": modality,
            "metadata": metadata or {},
            "created_at": time.time(),
        }

        await self.write(mem_path, json.dumps(memory_data, ensure_ascii=False))
        return {"id": memory_id, "status": "stored"}

    async def memory_search(
        self,
        query: str,
        limit: int = 10,
        modality: Optional[str] = None,
    ) -> list:
        """Search memory content.

        Uses POST /api/v1/memories/search when available, otherwise
        falls back to filesystem grep operations.

        Args:
            query: Search query string
            limit: Maximum number of results
            modality: Optional filter by modality type

        Returns:
            List of matching memory entries
        """
        # Try REST API first
        try:
            payload = {"query": query, "limit": limit}
            if modality:
                payload["modality"] = modality
            result = await self._request("POST", "/api/v1/memories/search", json=payload)
            return result.get("results", [])
        except Exception:
            pass

        # Fallback: grep through /memories filesystem
        import re

        matches = []
        try:
            entries = await self.ls("/memories")
            for entry in entries[:limit]:
                if entry.is_file:
                    try:
                        content = await self.cat(f"/memories/{entry.name}")
                        if isinstance(content, bytes):
                            content = content.decode("utf-8", errors="ignore")
                        if query.lower() in content.lower():
                            matches.append({"path": f"/memories/{entry.name}", "match": query})
                    except Exception:
                        continue
        except Exception:
            pass

        return matches

    async def memory_list(
        self,
        modality: Optional[str] = None,
        limit: int = 100,
    ) -> list:
        """List memory entries.

        Uses GET /api/v1/memories when available, otherwise
        falls back to filesystem listing.

        Args:
            modality: Optional filter by modality type
            limit: Maximum number of entries to return

        Returns:
            List of memory entries with metadata
        """
        # Try REST API first - note: API returns array directly, not wrapped object
        try:
            params = {"limit": limit}
            if modality:
                params["modality"] = modality
            result = await self._request("GET", "/api/v1/memories", params=params)
            # API returns array directly, not {"memories": [...]}
            if isinstance(result, list):
                return result[:limit]
            return result.get("memories", [])
        except Exception:
            pass

        # Fallback: list from /memories filesystem
        memories = []
        try:
            entries = await self.ls("/memories")
            count = 0
            for entry in entries:
                if count >= limit:
                    break
                if entry.is_file:
                    try:
                        content = await self.cat(f"/memories/{entry.name}")
                        if isinstance(content, bytes):
                            content = content.decode("utf-8", errors="ignore")
                        data = json.loads(content)
                        if modality is None or data.get("modality") == modality:
                            memories.append(data)
                            count += 1
                    except Exception:
                        continue
        except Exception:
            pass

        return memories

    async def memory_delete(self, memory_id: str) -> bool:
        """Delete a memory entry.

        Args:
            memory_id: ID of memory to delete

        Returns:
            True if successful
        """
        try:
            await self._request("DELETE", f"/api/v1/memories/{memory_id}")
            return True
        except Exception:
            pass

        # Fallback: delete from filesystem
        try:
            await self.rm(f"/memories/{memory_id}")
            return True
        except Exception:
            return False