# EVIF Claude Code Template

> EVIF (Everything Is a File) provides persistent context, reusable skills, and multi-agent coordination for AI agents.

## EVIF Skills

Claude Code can discover and use these EVIF skills:

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `evif-context` | Persistent memory across sessions | "read context", "remember this" |
| `evif-workflows` | Reusable task workflows | "run a skill", "code review" |
| `evif-pipes` | Multi-agent coordination | "send to another agent", "task queue" |
| `evif-memory` | Vector memory search | "search memories", "what do you know" |
| `evif-quickref` | Command quick reference | "evif help", "how do I use" |

Skills are stored in `.claude/skills/*.SKILL.md`.

## Context Layers

EVIF maintains three layers of context:

```
/context/
├── L0/current       # Current task (ephemeral)
├── L1/decisions.md # Session decisions (durable)
└── L2/             # Project knowledge (persistent)
    ├── architecture.md
    └── patterns.md
```

## Workflow

1. **Start of session**: Read context layers
   ```bash
   cat /context/L0/current           # What was I doing?
   cat /context/L1/decisions.md      # What decisions?
   ls /context/L2/                   # Project knowledge?
   ```

2. **During work**: Update context
   ```bash
   echo "Implementing feature X" > /context/L0/current
   echo "- $(date): Chose approach Y" >> /context/L1/decisions.md
   ```

3. **Skills**: Use built-in workflows
   ```bash
   evif ls /skills                   # Discover skills
   evif cat /skills/code-review/SKILL.md  # Read skill
   ```

4. **Coordination**: Multi-agent via pipes
   ```bash
   evif mkdir /pipes/my-task
   evif write -c "task description" /pipes/my-task/input
   ```

## CLI Reference

```bash
evif health              # Check server status
evif ls <path>           # List directory
evif cat <path>          # Read file
evif write -c <content> <path>  # Write file
evif mkdir <path>        # Create directory
evif rm <path>           # Remove
```

## Example Session

```bash
# 1. Check where I left off
cat /context/L0/current
cat /context/L1/decisions.md

# 2. Review project patterns
cat /context/L2/architecture.md

# 3. Use a skill for code review
evif write -c "Review src/auth/" /skills/code-review/input
sleep 1 && cat /skills/code-review/output

# 4. Coordinate with another agent
evif mkdir /pipes/review-task
evif write -c "Review PR #123" /pipes/review-task/input

# 5. Update context when done
echo "Completed code review for PR #123" > /context/L0/current
```

## Guardrails

- **L0**: One line, current task only
- **L1**: Session notes, decisions with reasons
- **L2**: Stable docs (architecture, patterns, runbooks)
- **Skills**: Read SKILL.md before executing
- **Pipes**: Use descriptive names (`/pipes/review-pr-123` not `/pipes/task1`)
