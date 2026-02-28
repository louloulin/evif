# EVIF 1.8 使用示例

本目录包含EVIF 1.8的完整使用示例。

## 快速开始

### 基础文件操作
```bash
evif ls /
evif cat /memfs/test.txt
evif write /memfs/hello.txt "Hello, EVIF!"
```

### S3存储操作
```bash
evif mount s3fs /s3fs
evif upload local.txt /s3fs/bucket/
evif download /s3fs/bucket/data.txt ./
```

### 消息队列
```bash
evif mkdir /queuefs/tasks
echo "Task data" | evif write /queuefs/tasks/enqueue -
evif cat /queuefs/tasks/dequeue
```

更多示例请查看后续文档...
