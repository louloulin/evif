# EVIF + Codex Workflow

## Bootstrap Files

- [`AGENTS.md`](/Users/louloulin/Documents/linchong/claude/evif/AGENTS.md)
- [`agents/openai.yaml`](/Users/louloulin/Documents/linchong/claude/evif/agents/openai.yaml)

## Recommended Startup

```bash
cat /context/L0/current
cat /context/L1/decisions.md
ls /skills
ls /pipes
```

## Practical Pattern

1. Read the current context.
2. Use `/skills/<name>/SKILL.md` to understand available workflows.
3. Write to `/skills/<name>/input` when a skill should execute.
4. Read `/skills/<name>/output` for the result.
5. Use `/pipes/<task>/status` to track cross-agent work.
