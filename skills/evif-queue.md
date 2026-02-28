---
name: evif-queue
description: "EVIF 消息队列技能 - QueueFS完整使用指南"
parent: "evif"
tags: ["evif", "queue", "message-queue", "task-queue", "pubsub"]
trigger_keywords: ["evif queue", "消息队列", "任务队列", "enqueue", "dequeue"]
---

# EVIF 消息队列

本文档详细介绍 EVIF QueueFS 的消息队列功能,支持多种后端 (内存、SQLite、TiDB) 和丰富的操作模式。

## 核心概念

QueueFS 提供类 Unix 文件接口的消息队列系统:
- **生产者-消费者模式**: 入队 (enqueue) / 出队 (dequeue)
- **持久化支持**: SQLite/TiDB 后端保证数据不丢失
- **多队列支持**: 支持嵌套队列 (如 `/queue/logs/errors`)
- **原子操作**: 保证消息操作的一致性
- **多后端**: Memory (快速)、SQLite (单机)、TiDB (分布式)

## 基础操作

### 1. 挂载 QueueFS

**内存后端 (默认,快速但易失):**
```bash
evif mount queuefs /queue --backend=memory
```

**SQLite 后端 (持久化,单机):**
```bash
evif mount queuefs /queue \
  --backend=sqlite \
  --db-path=/var/lib/evif/queue.db
```

**TiDB 后端 (分布式,生产):**
```bash
evif mount queuefs /queue \
  --backend=tidb \
  --tidb-host=localhost:4000 \
  --tidb-user=root \
  --tidb-password=yourpassword \
  --tidb-database=queuefs
```

### 2. 基础队列操作

#### 入队 (Enqueue)

**单条消息:**
```bash
# 方法1: 直接写入
echo "Hello, Queue!" | evif write /queue/myqueue/enqueue

# 方法2: 从文件
evif write /queue/myqueue/enqueue --file=message.txt

# 方法3: 直接传参
evif write /queue/myqueue/enqueue "Task data"
```

**批量入队:**
```bash
# 从文件批量入队
cat messages.txt | while read line; do
    echo "$line" | evif write /queue/myqueue/enqueue
done

# 或使用批量操作
evif write /queue/myqueue/batch-enqueue --file=messages.txt
```

**入队带元数据:**
```bash
# JSON 格式消息
evif write /queue/myqueue/enqueue '{
  "type": "email",
  "to": "user@example.com",
  "subject": "Hello",
  "body": "Message content"
}'
```

#### 出队 (Dequeue)

**出队一条消息:**
```bash
# FIFO (先进先出)
MESSAGE=$(evif cat /queue/myqueue/dequeue)
echo "Received: $MESSAGE"
```

**阻塞式出队 (等待消息):**
```bash
# 阻塞等待,直到有消息
evif cat /queue/myqueue/dequeue --block

# 带超时的阻塞
evif cat /queue/myqueue/dequeue --block --timeout=30
```

**批量出队:**
```bash
# 一次出队多条消息
evif cat /queue/myqueue/dequeue --count=10
```

#### 查看队列状态

**队列大小:**
```bash
SIZE=$(evif cat /queue/myqueue/size)
echo "Queue size: $SIZE"
```

**查看队首 (不出队):**
```bash
evif cat /queue/myqueue/peek
```

**队列统计:**
```bash
evif stat /queue/myqueue
```

输出:
```
Queue: myqueue
Size: 1234 messages
Backend: sqlite
Created: 2025-01-25 10:00:00
Last activity: 2025-01-25 12:30:45
```

#### 清空队列

```bash
# 清空所有消息
evif write /queue/myqueue/clear "clear"

# 删除队列
evif rm /queue/myqueue
```

## 高级功能

### 1. 嵌套队列

**场景: 按类型组织队列**
```bash
# 创建嵌套队列
evif write /queue/tasks/email/enqueue "send email to user1"
evif write /queue/tasks/sms/enqueue "send sms to user2"
evif write /queue/tasks/push/enqueue "send push notification"

# 查看所有任务队列
evif ls /queue/tasks/

# 统计所有任务
for queue in $(evif ls /queue/tasks/); do
    size=$(evif cat "/queue/tasks/$queue/size")
    echo "$queue: $size messages"
done
```

### 2. 优先级队列

**使用不同的优先级:**
```bash
# 高优先级队列
evif write /queue/tasks/high/enqueue "urgent task"

# 普通优先级队列
evif write /queue/tasks/normal/enqueue "normal task"

# 低优先级队列
evif write /queue/tasks/low/enqueue "background task"

# 消费时按优先级处理
while true; do
    # 先处理高优先级
    if [ "$(evif cat /queue/tasks/high/size)" -gt "0" ]; then
        evif cat /queue/tasks/high/dequeue
    elif [ "$(evif cat /queue/tasks/normal/size)" -gt "0" ]; then
        evif cat /queue/tasks/normal/dequeue
    else
        evif cat /queue/tasks/low/dequeue
    fi
done
```

### 3. 延迟队列

**使用时间戳实现延迟:**
```bash
# 入队时添加执行时间
evif write /queue/delayed/enqueue '{
  "task": "send_email",
  "execute_at": "2025-01-25T15:00:00Z",
  "data": "..."
}'

# 消费者检查时间
while true; do
    MESSAGE=$(evif cat /queue/delayed/peek)
    EXECUTE_AT=$(echo "$MESSAGE" | jq -r '.execute_at')
    NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    if [ "$EXECUTE_AT" \< "$NOW" ]; then
        # 时间到了,出队处理
        evif cat /queue/delayed/dequeue
    else
        # 还没到时间,等待
        sleep 10
    fi
done
```

### 4. 死信队列 (DLQ)

**自动重试失败:**
```bash
# 主队列
MAIN_QUEUE=/queue/tasks/main
# 重试队列
RETRY_QUEUE=/queue/tasks/retry
# 死信队列
DLQ_QUEUE=/queue/tasks/dead

MAX_RETRIES=3

# 消费者逻辑
while true; do
    MESSAGE=$(evif cat $MAIN_QUEUE/dequeue)

    # 尝试处理
    if process_message "$MESSAGE"; then
        echo "Success"
    else
        # 处理失败
        RETRY_COUNT=$(echo "$MESSAGE" | jq -r '.retry_count // 0')

        if [ "$RETRY_COUNT" -lt "$MAX_RETRIES" ]; then
            # 增加重试计数
            NEW_MESSAGE=$(echo "$MESSAGE" | jq ".retry_count += 1")
            echo "$NEW_MESSAGE" | evif write $RETRY_QUEUE/enqueue
            echo "Retrying ($RETRY_COUNT/$MAX_RETRIES)"
        else
            # 超过最大重试次数,进入死信队列
            echo "$MESSAGE" | evif write $DLQ_QUEUE/enqueue
            echo "Moved to DLQ"
        fi
    fi
done
```

### 5. 广播队列

**多消费者模式:**
```bash
# 生产者: 向广播队列发送消息
evif write /queue/broadcast/message1 "Important announcement"

# 消费者1: 订阅广播
evif write /queue/broadcast/subscribers/consumer1 "subscribe"

# 消费者2: 订阅广播
evif write /queue/broadcast/subscribers/consumer2 "subscribe"

# 广播消息到所有订阅者
MESSAGE=$(evif cat /queue/broadcast/message1)
for consumer in $(evif ls /queue/broadcast/subscribers/); do
    echo "$MESSAGE" | evif write "/queue/broadcast/subscribers/$consumer/enqueue"
done
```

## 使用场景

### 场景1: 异步任务处理

**生产者 (提交任务):**
```bash
# 提交异步任务
submit_task() {
    local task_data="$1"
    echo "$task_data" | evif write /queue/tasks/enqueue
    echo "Task submitted: $task_data"
}

# 批量提交
for i in {1..100}; do
    submit_task "{\"task_id\": $i, \"type\": \"process\", \"data\": \"...\"}"
done
```

**消费者 (处理任务):**
```bash
# 启动多个消费者进程
worker() {
    local worker_id=$1
    echo "Worker $worker_id started"

    local processed=0
    while true; do
        # 出队任务
        TASK=$(evif cat /queue/tasks/dequeue --block)

        if [ -n "$TASK" ]; then
            echo "Worker $worker_id processing: $TASK"

            # 处理任务
            if process_task "$TASK"; then
                ((processed++))
                echo "Worker $worker_id processed $processed tasks"
            else
                echo "Worker $worker_id failed to process task"
            fi
        fi
    done
}

# 启动5个并发消费者
for i in {1..5}; do
    worker $i &
done
```

### 场景2: 日志收集系统

**日志生产者:**
```bash
# 应用日志写入队列
app_log() {
    local level=$1
    local message=$2
    local timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    echo "[$timestamp] [$level] $message" | evif write /queue/logs/$level/enqueue
}

# 使用
app_log "info" "Application started"
app_log "error" "Database connection failed"
app_log "warning" "High memory usage"
```

**日志消费者:**
```bash
# 根据级别路由日志
route_logs() {
    while true; do
        # 处理错误日志 (高优先级)
        if [ "$(evif cat /queue/logs/error/size)" -gt "0" ]; then
            LOG=$(evif cat /queue/logs/error/dequeue)
            echo "$LOG" | evif write /local/logs/error.log --append
            # 发送告警
            send_alert "$LOG"
        fi

        # 处理警告日志
        if [ "$(evif cat /queue/logs/warning/size)" -gt "0" ]; then
            LOG=$(evif cat /queue/logs/warning/dequeue)
            echo "$LOG" | evif write /local/logs/warning.log --append
        fi

        # 处理普通日志
        if [ "$(evif cat /queue/logs/info/size)" -gt "0" ]; then
            LOG=$(evif cat /queue/logs/info/dequeue)
            echo "$LOG" | evif write /local/logs/app.log --append
        fi

        sleep 1
    done
}

route_logs &
```

### 场景3: 工作流引擎

**定义工作流:**
```bash
# 工作流: 数据处理流水线
WORKFLOW_STAGES=(
    "validate"
    "transform"
    "enrich"
    "index"
)

# 提交工作流任务
submit_workflow() {
    local data="$1"
    local workflow_id=$(uuidgen)

    echo "Starting workflow: $workflow_id"

    # 初始数据入队
    echo "$data" | evif write "/queue/workflows/$workflow_id/input/enqueue"

    # 提交到第一阶段
    echo "{\"workflow_id\": \"$workflow_id\", \"stage\": \"validate\", \"data\": \"$data\"}" | \
        evif write /queue/workflows/validate/enqueue
}

# 工作流处理器
process_workflow_stage() {
    local stage=$1

    while true; do
        TASK=$(evif cat "/queue/workflows/$stage/dequeue" --block)

        # 解析任务
        WORKFLOW_ID=$(echo "$TASK" | jq -r '.workflow_id')
        DATA=$(echo "$TASK" | jq -r '.data')
        CURRENT_STAGE=$(echo "$TASK" | jq -r '.stage')

        echo "Processing $CURRENT_STAGE for workflow $WORKFLOW_ID"

        # 处理当前阶段
        RESULT=$(process_stage_$CURRENT_STAGE "$DATA")

        # 查找下一阶段
        NEXT_STAGE=$(find_next_stage "$CURRENT_STAGE")

        if [ -n "$NEXT_STAGE" ]; then
            # 提交到下一阶段
            echo "{\"workflow_id\": \"$WORKFLOW_ID\", \"stage\": \"$NEXT_STAGE\", \"data\": \"$RESULT\"}" | \
                evif write "/queue/workflows/$NEXT_STAGE/enqueue"
        else
            # 工作流完成
            echo "$RESULT" | evif write "/queue/workflows/$workflow_id/output/enqueue"
            echo "Workflow $WORKFLOW_ID completed"
        fi
    done
}

# 启动所有阶段的处理器
for stage in "${WORKFLOW_STAGES[@]}"; do
    process_workflow_stage "$stage" &
done
```

### 场景4: 事件驱动架构

**事件发布者:**
```bash
# 发布事件
publish_event() {
    local event_type=$1
    local event_data=$2
    local timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    echo "{\"type\": \"$event_type\", \"timestamp\": \"$timestamp\", \"data\": $event_data}" | \
        evif write /queue/events/$event_type/enqueue

    echo "Event published: $event_type"
}

# 使用示例
publish_event "user.created" '{"user_id": 123, "email": "user@example.com"}'
publish_event "order.placed" '{"order_id": 456, "amount": 99.99}'
```

**事件订阅者:**
```bash
# 订阅特定类型的事件
subscribe_event() {
    local event_type=$1
    local handler=$2

    while true; do
        # 从特定事件队列出队
        EVENT=$(evif cat "/queue/events/$event_type/dequeue" --block)

        echo "Received $event_type event: $EVENT"

        # 调用处理器
        $handler "$EVENT"
    done
}

# 定义事件处理器
handle_user_created() {
    local event=$1
    local user_id=$(echo "$event" | jq -r '.data.user_id')
    local email=$(echo "$event" | jq -r '.data.email')

    echo "Sending welcome email to $email"
    send_welcome_email "$email"

    # 触发后续事件
    publish_event "welcome.email.sent" "{\"user_id\": $user_id}"
}

handle_order_placed() {
    local event=$1
    local order_id=$(echo "$event" | jq -r '.data.order_id')
    local amount=$(echo "$event" | jq -r '.data.amount')

    echo "Processing order $order_id for amount $amount"
    process_order "$order_id"
}

# 订阅多个事件
subscribe_event "user.created" handle_user_created &
subscribe_event "order.placed" handle_order_placed &
subscribe_event "payment.received" handle_payment &
```

### 场景5: 批处理调度

**定时批处理:**
```bash
# 每5分钟收集一批任务
collect_and_process_batch() {
    local batch_size=100
    local batch_file="/tmp/batch_$(date +%s).txt"

    # 收集任务
    local count=0
    while [ $count -lt $batch_size ]; do
        # 检查是否有任务
        SIZE=$(evif cat /queue/batch/input/size)

        if [ "$SIZE" -eq "0" ]; then
            break
        fi

        # 出队并保存
        TASK=$(evif cat /queue/batch/input/dequeue)
        echo "$TASK" >> "$batch_file"
        ((count++))
    done

    # 如果收集到任务,处理批次
    if [ $count -gt 0 ]; then
        echo "Processing batch of $count tasks"

        # 处理批次
        process_batch "$batch_file"

        # 清理临时文件
        rm "$batch_file"
    else
        echo "No tasks to process"
    fi
}

# 每5分钟运行一次
while true; do
    collect_and_process_batch
    sleep 300
done
```

## 性能优化

### 1. 批量操作

**批量入队优化:**
```bash
# 使用事务批量入队 (SQLite/TiDB)
evif write /queue/tasks/batch-enqueue \
  --file=messages.txt \
  --batch-size=1000
```

### 2. 连接池配置

**TiDB 连接池:**
```toml
[plugins.queuefs]
backend = "tidb"
connection_pool_size = 10
max_idle_connections = 5
connection_lifetime = 3600
```

### 3. 队列分区

**按类型分区:**
```bash
# 将任务分散到多个队列
for i in {0..9}; do
    echo "$TASK_DATA" | evif write "/queue/tasks/partition_$i/enqueue"
done

# 消费者随机选择分区
PARTITION=$((RANDOM % 10))
evif cat "/queue/tasks/partition_$PARTITION/dequeue"
```

### 4. 索引优化

**TiDB 索引:**
```sql
-- 为队列创建索引
CREATE INDEX idx_queue_name ON messages(queue_name);
CREATE INDEX idx_created_at ON messages(created_at);
CREATE INDEX idx_status ON messages(status);
```

## 监控和管理

### 队列监控

```bash
# 查看所有队列统计
evif write /queue/stats/all ""

# 实时监控
watch -n 1 'evif stat /queue/tasks'
```

### 性能指标

```bash
# 吞吐量统计
evif cat /queue/stats/throughput

# 延迟统计
evif cat /queue/stats/latency

# 错误率
evif cat /queue/stats/errors
```

## 故障排查

### 常见问题

**1. 队列满**
```bash
# 检查队列限制
evif cat /queue/tasks/limits

# 增加队列大小
evif write /queue/tasks/config "max_size=10000"
```

**2. 消费速度慢**
```bash
# 增加消费者数量
for i in {1..10}; do
    worker $i &
done

# 查看消费者状态
evif cat /queue/tasks/consumers
```

**3. 消息丢失**
```bash
# 检查后端持久化
evif cat /queue/config/backend

# 切换到持久化后端
evif mount queuefs /queue --backend=sqlite --db-path=/var/lib/evif/queue.db
```

## 配置示例

### 完整配置文件

```toml
[plugins.queuefs]
backend = "sqlite"
db_path = "/var/lib/evif/queue.db"

[plugins.queuefs.queue]
max_size = 10000
message_ttl = 86400  # 24 hours
retention_policy = "delete_after_consumption"

[plugins.queuefs.performance]
enable_batch = true
batch_size = 100
flush_interval = 5  # seconds

[plugins.queuefs.monitoring]
enable_metrics = true
metrics_port = 9090
```

## 最佳实践

### 1. 消息格式

**使用标准JSON格式:**
```json
{
  "id": "uuid-or-int",
  "timestamp": "2025-01-25T10:00:00Z",
  "type": "task_type",
  "data": { ... },
  "metadata": {
    "priority": "high",
    "retry_count": 0,
    "timeout": 300
  }
}
```

### 2. 错误处理

**消费者错误处理:**
```bash
while true; do
    MESSAGE=$(evif cat /queue/tasks/dequeue --block)

    if process_message "$MESSAGE"; then
        # 成功,确认完成
        echo "OK"
    else
        # 失败,记录错误并可能重试
        log_error "$MESSAGE"

        # 检查是否应该重试
        SHOULD_RETRY=$(echo "$MESSAGE" | jq -r '.metadata.retry // true')

        if [ "$SHOULD_RETRY" = "true" ]; then
            # 重新入队
            echo "$MESSAGE" | evif write /queue/tasks/enqueue
        else
            # 移到死信队列
            echo "$MESSAGE" | evif write /queue/tasks/dlq/enqueue
        fi
    fi
done
```

### 3. 优雅关闭

**信号处理:**
```bash
worker() {
    trap "graceful_shutdown" SIGTERM SIGINT

    while true; do
        # 检查停止标志
        if [ -f "/tmp/worker.stop" ]; then
            echo "Shutting down gracefully..."
            break
        fi

        # 正常处理
        TASK=$(evif cat /queue/tasks/dequeue --timeout=1)

        if [ -n "$TASK" ]; then
            process_task "$TASK"
        fi
    done

    # 清理
    cleanup
}

graceful_shutdown() {
    echo "Received shutdown signal"
    touch /tmp/worker.stop
}
```

---

**相关技能:**
- `SKILL.md` - EVIF 主技能
- `evif-manage.md` - 插件管理
- `evif-gpt.md` - AI任务处理
