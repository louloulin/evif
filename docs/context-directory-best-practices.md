# Context Directory Best Practices

## Layer Intent

- `L0`: only the smallest possible working set for the current task.
- `L1`: session decisions, temporary notes, and drafts that may span several tool calls.
- `L2`: stable project knowledge that should survive across sessions.

## Recommended Usage

```bash
cat /context/L0/current
cat /context/L1/decisions.md
grep "auth" /context/L2/patterns.md
```

- Keep `L0/current` short enough to read in one glance.
- Move anything durable out of `L0` quickly.
- Use `L1/scratch/` for temporary notes and `L1/intermediate/` for artifacts.
- Treat `L2/architecture.md` and `L2/patterns.md` as the canonical project memory.

## Promotion Rules

- `L0 -> L1`: when information matters for the rest of the session.
- `L1 -> L2`: when the note should help future sessions or other agents.
- Delete or rewrite stale `L0` entries instead of letting them drift.
