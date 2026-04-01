"""EVIF Skill API - Skill discovery and execution."""

import json
from typing import Optional


class SkillApi:
    """Mixin providing Skill API operations for EvifClient."""

    async def skill_discover(self) -> list:
        """List available skills."""
        entries = await self.ls("/skills")
        return [e.name for e in entries if e.is_dir]

    async def skill_read(self, name: str) -> str:
        """Read a skill's SKILL.md."""
        data = await self.cat(f"/skills/{name}/SKILL.md")
        return data.decode("utf-8") if isinstance(data, bytes) else str(data)

    async def skill_execute(self, name: str, input_data: str) -> str:
        """Execute a skill by writing input and reading output."""
        await self.write(f"/skills/{name}/input", input_data)
        data = await self.cat(f"/skills/{name}/output")
        return data.decode("utf-8") if isinstance(data, bytes) else str(data)

    async def skill_register(self, name: str, skill_md: str) -> bool:
        """Register a new skill by creating directory and writing SKILL.md."""
        await self.mkdir(f"/skills/{name}")
        await self.write(f"/skills/{name}/SKILL.md", skill_md)
        return True

    async def skill_match(self, query: str) -> Optional[str]:
        """Find a skill matching the query by checking triggers."""
        skills = await self.skill_discover()
        query_lower = query.lower()
        for skill_name in skills:
            try:
                content = await self.skill_read(skill_name)
                # Parse YAML frontmatter to get triggers
                if "triggers:" in content:
                    triggers_section = False
                    for line in content.split("\n"):
                        if line.strip() == "triggers:":
                            triggers_section = True
                            continue
                        if triggers_section:
                            stripped = line.strip()
                            if stripped.startswith("- "):
                                trigger = (
                                    stripped[2:].strip().strip('"').strip("'")
                                )
                                if trigger.lower() in query_lower:
                                    return skill_name
                            elif not stripped.startswith("-") and not stripped:
                                triggers_section = False
                                continue
            except Exception:
                continue
        return None

    async def skill_remove(self, name: str) -> bool:
        """Remove a skill."""
        await self.rm(f"/skills/{name}", recursive=True)
        return True
