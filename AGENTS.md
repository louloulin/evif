# EVIF Codex Template

## Operating Model

EVIF exposes agent context through files. Start with:

```bash
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills
ls /pipes
```

## Conventions

- Update `/context/L0/current` with the active objective.
- Append important choices to `/context/L1/decisions.md`.
- Save reusable architecture notes under `/context/L2/`.
- Use `/skills/<skill>/SKILL.md` for discoverable agent workflows.
- Use `/pipes/<task>/input|output|status` for agent coordination.

## Recommended Loop

1. Read current context.
2. Inspect relevant code and `/context/L2` notes.
3. Execute changes.
4. Record decisions back into `/context/L1/decisions.md`.
5. If coordination is needed, create a pipe under `/pipes/`.
