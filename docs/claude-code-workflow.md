# EVIF + Claude Code Workflow

## FUSE Mount Example

```bash
cargo build --release -p evif-rest -p evif-fuse
./target/release/evif-rest --port 8081
./target/release/evif-fuse-mount /tmp/evif --readwrite
```

## Claude Code Bootstrap

1. Place the project template from [`CLAUDE.md`](/Users/louloulin/Documents/linchong/claude/evif/CLAUDE.md) at the repo root.
2. Start by reading `/context/L0/current` and `/context/L1/decisions.md`.
3. Discover reusable workflows with `ls /skills`.
4. Use `/pipes` when another agent or process should pick up work asynchronously.

## Example Flow

```bash
cat /tmp/evif/context/L0/current
cat /tmp/evif/context/L1/decisions.md
ls /tmp/evif/skills
cat /tmp/evif/skills/code-review/SKILL.md
mkdir /tmp/evif/pipes/task-001
echo "review api handlers" > /tmp/evif/pipes/task-001/input
```
