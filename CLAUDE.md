# EVIF Claude Code Template

## Mission

Treat EVIF as a context filesystem for AI agents. Prefer file primitives first:

1. Read `/context/L0/current` for the active task.
2. Review `/context/L1/decisions.md` before making changes.
3. Search `/context/L2/` for architecture or pattern guidance.
4. Discover reusable workflows under `/skills/`.
5. Coordinate with other agents through `/pipes/` when collaboration is needed.

## Workflow

- Keep new task state in `/context/L0/current`.
- Record durable decisions in `/context/L1/decisions.md`.
- Promote reusable knowledge into `/context/L2/architecture.md` or `/context/L2/patterns.md`.
- Prefer `ls`, `cat`, `grep`, and `write` over bespoke tools when EVIF exposes the same surface.

## Example Session

```bash
cat /context/L0/current
cat /context/L1/decisions.md
grep "mount" /context/L2/patterns.md
ls /skills
cat /skills/code-review/SKILL.md
mkdir /pipes/task-001
echo "review changed files" > /pipes/task-001/input
```

## Guardrails

- Keep `/context/L0` short and task-focused.
- Use `/context/L1` for session notes, not long-term docs.
- Store stable project knowledge under `/context/L2`.
- Use `/pipes/broadcast` only for information that should fan out to multiple subscribers.
