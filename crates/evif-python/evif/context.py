"""EVIF Context API - L0/L1/L2 context management."""

from typing import Optional


class ContextApi:
    """Mixin providing Context API operations for EvifClient."""

    async def context_read(self, path: str) -> str:
        """Read a context file from /context/{path}."""
        data = await self.cat(f"/context/{path}")
        return data.decode("utf-8") if isinstance(data, bytes) else str(data)

    async def context_write(self, path: str, content: str) -> int:
        """Write to a context file at /context/{path}."""
        return await self.write(f"/context/{path}", content)

    async def context_list(self, layer: str = "") -> list:
        """List files in context layer. layer can be 'L0', 'L1', 'L2', or '' for root."""
        prefix = f"/context/{layer}" if layer else "/context"
        return await self.ls(prefix)

    async def context_current(self) -> str:
        """Read L0 current context."""
        return await self.context_read("L0/current")

    async def context_update_current(self, context: str) -> int:
        """Update L0 current context."""
        return await self.context_write("L0/current", context)

    async def context_decisions(self) -> str:
        """Read L1 decisions."""
        return await self.context_read("L1/decisions.md")

    async def context_add_decision(self, decision: str) -> int:
        """Append a decision to L1/decisions.md."""
        existing = await self.context_decisions()
        new_content = existing.rstrip() + f"\n- {decision}\n"
        return await self.context_write("L1/decisions.md", new_content)

    async def context_recent_ops(self) -> list:
        """Read L0 recent operations."""
        import json

        raw = await self.context_read("L0/recent_ops")
        return json.loads(raw)

    async def context_search(self, query: str, layer: Optional[str] = None) -> list:
        """Search context files using grep."""
        path = f"/context/{layer}" if layer else "/context"
        return await self.grep(path, query, recursive=True)

    async def context_meta(self) -> dict:
        """Read context metadata."""
        import json

        raw = await self.context_read(".meta")
        return json.loads(raw)

    async def context_knowledge(self, name: str) -> str:
        """Read a L2 knowledge file."""
        return await self.context_read(f"L2/{name}")

    async def context_add_knowledge(self, name: str, content: str) -> int:
        """Write a L2 knowledge file."""
        return await self.context_write(f"L2/{name}", content)
