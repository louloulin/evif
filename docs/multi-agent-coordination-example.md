# Multi-Agent Coordination Example

## PipeFS + ContextFS

```bash
mkdir /pipes/task-001
echo "analyze auth flow" > /pipes/task-001/input
echo "agent-b" > /pipes/task-001/assignee
cat /pipes/task-001/status
```

## Shared Context

```bash
echo "task-001 assigned to agent-b" >> /context/L1/decisions.md
echo "auth subsystem is under review" > /context/L0/current
```

## Broadcast Update

```bash
mkdir /pipes/broadcast/subscribers/agent-a
mkdir /pipes/broadcast/subscribers/agent-b
echo "new architecture decision available" > /pipes/broadcast/input
cat /pipes/broadcast/subscribers/agent-a/output
cat /pipes/broadcast/subscribers/agent-b/output
```
