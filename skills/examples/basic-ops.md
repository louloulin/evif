# EVIF 基础操作示例

本文档提供 EVIF 文件系统的基础操作示例。

## 准备工作

### 1. 安装 EVIF

```bash
# 从源码编译
git clone https://github.com/evif/evif.git
cd evif
cargo build --release
sudo cp target/release/evif /usr/local/bin/
sudo cp target/release/evif-server /usr/local/bin/
```

### 2. 启动 EVIF 服务器

```bash
evif-server --config ~/.evif/config.toml
```

### 3. 验证安装

```bash
# 检查版本
evif --version

# 测试连接
evif health
```

## 基础文件操作

### 列出文件

```bash
# 列出根目录
evif ls /

# 列出本地文件系统根目录
evif ls /local/

# 详细信息
evif ls /local/projects --detailed

# 限制数量
evif ls /local/data/ --limit=10

# 递归列出
evif ls /local/projects/ --recursive
```

### 读取文件

```bash
# 读取整个文件
evif cat /local/README.md

# 读取指定偏移量
evif cat /local/large-file.bin --offset=1024 --size=4096

# 读取并保存到本地
evif cat /s3/my-bucket/data.json > local-data.json

# 查看前 N 行
evif cat /local/log.txt | head -n 10

# 查看后 N 行
evif cat /local/log.txt | tail -n 10
```

### 写入文件

```bash
# 写入文本
evif write /local/test.txt "Hello, EVIF!"

# 从文件写入
evif write /local/output.txt --file=input.txt

# 从标准输入写入
echo "Line 1" | evif write /local/log.txt
echo "Line 2" | evif write /local/log.txt --append

# 追加模式
evif write /local/data.txt "new data" --append

# 创建新文件 (排他)
evif write /local/new.txt "content" --create-exclusive
```

### 创建目录

```bash
# 创建单个目录
evif mkdir /local/projects

# 递归创建
evif mkdir /local/projects/evif/src --parents

# 指定权限
evif mkdir /local/data --mode=0o755
```

### 删除文件

```bash
# 删除文件
evif rm /local/test.txt

# 删除空目录
evif rm /local/empty-dir/

# 递归删除目录
evif rm /local/old-projects/ --recursive

# 强制删除
evif rm /local/data.txt --force
```

### 移动和重命名

```bash
# 重命名文件
evif mv /local/old.txt /local/new.txt

# 移动文件
evif mv /local/data.txt /local/archive/data.txt

# 移动目录
evif mv /local/old-projects /local/archive/projects
```

### 复制文件

```bash
# 复制文件
evif cp /local/source.txt /local/destination.txt

# 复制目录 (递归)
evif cp /local/projects/ /local/backup/projects/ --recursive

# 跨插件复制
evif cp /s3/my-bucket/data.json /local/backup/data.json
```

### 文件信息

```bash
# 获取文件信息
evif stat /local/README.md

# JSON 格式输出
evif stat /local/data.txt --json

# 人类可读格式
evif stat /local/large-file.bin --human-readable
```

输出示例:
```
File: /local/README.md
Size: 1234 bytes
Modified: 2025-01-25 10:30:45 UTC
Mode: 0644
Type: Regular File
```

## 插件操作

### 挂载插件

```bash
# 挂载本地文件系统
evif mount localfs /local --storage-path=/home/user/data

# 挂载 S3
evif mount s3fs /s3 \
  --region=us-east-1 \
  --bucket=my-bucket \
  --access-key=AKIAIOSFODNN7EXAMPLE \
  --secret-key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# 挂载内存文件系统
evif mount memfs /mem

# 挂载键值存储
evif mount kvfs /kv --storage-path=/var/lib/evif/kv.db
```

### 卸载插件

```bash
# 卸载指定挂载点
evif unmount /local

# 强制卸载
evif unmount /s3 --force

# 卸载所有
evif unmount --all
```

### 列出挂载点

```bash
# 列出所有挂载点
evif mounts

# 详细信息
evif mounts --verbose

# JSON 格式
evif mounts --json
```

## 高级操作

### Grep 搜索

```bash
# 在文件中搜索
evif grep /local/projects/ "TODO" --limit=10

# 正则表达式
evif grep /local/code/ "fn\s+\w+" --regex

# 递归搜索
evif grep /local/projects/ "EVIF" --recursive

# 仅显示文件名
evif grep /local/code/ "async" --files-only

# 显示行号
evif grep /local/log.txt "ERROR" --line-number
```

### 使用文件句柄 (大文件处理)

```bash
# 打开文件句柄
HANDLE_ID=$(evif handle open /local/large-file.bin)

# 读取数据
evif handle read $HANDLE_ID --offset=0 --size=4096

# 写入数据
evif handle write $HANDLE_ID --data="$(cat chunk.bin)"

# 获取句柄信息
evif handle info $HANDLE_ID

# 关闭句柄
evif handle close $HANDLE_ID
```

### 批量操作

```bash
# 批量删除 .tmp 文件
evif ls /local/data/ | grep ".tmp$" | while read f; do
    evif rm "/local/data/$f"
done

# 批量复制
for file in $(evif ls /local/src/); do
    evif cp "/local/src/$file" "/s3/backup/$file"
done

# 批量重命名
counter=1
for file in $(evif ls /local/images/ | grep ".jpg$"); do
    evif mv "/local/images/$file" "/local/images/img_$counter.jpg"
    counter=$((counter + 1))
done
```

## 实际应用场景

### 场景1: 日志分析

```bash
# 查找错误日志
evif grep /local/logs/ "ERROR" --recursive

# 查找特定时间的日志
evif cat /local/logs/app.log | grep "2025-01-25"

# 统计错误数量
evif grep /local/logs/ "ERROR" --recursive --count

# 查找并保存到文件
evif grep /local/logs/ "CRITICAL" --recursive > errors.txt
```

### 场景2: 备份到 S3

```bash
# 备份单个文件
evif cp /local/important.txt /s3/backups/important.txt

# 备份目录
evif cp /local/projects/ /s3/backups/projects/ --recursive

# 定时备份
while true; do
    evif cp /local/data/ /s3/backups/data-$(date +%Y%m%d)/ --recursive
    sleep 86400  # 每天备份一次
done
```

### 场景3: 数据处理管道

```bash
# 从 S3 读取数据,处理,保存到本地
evif cat /s3/raw-data/data.json | \
  jq '.[] | select(.status == "active")' | \
  evif write /local/filtered-data.json

# 处理多个文件
for file in $(evif ls /s3/input/); do
    evif cat "/s3/input/$file" | \
      python process.py | \
      evif write "/local/output/$file"
done
```

### 场景4: 监控文件变化

```bash
# 定期检查文件大小
while true; do
    evif stat /local/log.txt --human-readable
    sleep 60
done

# 监控目录变化
previous=$(evif ls /local/data/)
while true; do
    current=$(evif ls /local/data/)
    diff <(echo "$previous") <(echo "$current")
    previous=$current
    sleep 10
done
```

## 性能技巧

### 1. 使用 HandleFS 处理大文件

```bash
# ❌ 不推荐: 一次性读取整个大文件
evif cat /local/huge-file.bin

# ✅ 推荐: 分块读取
HANDLE_ID=$(evif handle open /local/huge-file.bin)
evif handle read $HANDLE_ID --offset=0 --size=1048576  # 1MB
evif handle read $HANDLE_ID --offset=1048576 --size=1048576
evif handle close $HANDLE_ID
```

### 2. 并行操作

```bash
# 使用 xargs 并行处理
evif ls /local/data/ | parallel -j 4 'evif cat /local/data/{} | wc -l'

# 使用多个进程
for dir in dir1 dir2 dir3 dir4; do
    (evif cp "/local/$dir/" "/s3/backup/$dir/" --recursive) &
done
wait
```

### 3. 利用缓存

```bash
# 第一次读取 (缓存未命中)
evif cat /s3/data.txt

# 后续读取 (缓存命中)
evif cat /s3/data.txt  # 更快
```

## 错误处理

### 检查错误

```bash
# 检查文件是否存在
evif stat /local/file.txt 2>&1 | grep -q "not found" && echo "File not found"

# 重试失败的操作
for i in {1..3}; do
    evif write /local/output.txt "data" && break
    sleep 1
done
```

### 清理资源

```bash
# 关闭所有打开的句柄
for handle in $(evif handle list); do
    evif handle close $handle
done

# 卸载所有插件
evif unmount --all
```

## 日常使用技巧

### 快速导航

```bash
# 创建快捷方式别名
alias evl='evif ls'
alias evc='evif cat'
alias evw='evif write'

# 快速切换目录
cd() {
    if [[ $1 == /* ]]; then
        echo $1
    else
        echo "/local/$1"
    fi | xargs evif cd
}
```

### 历史记录

```bash
# 查看操作历史
evif history

# 执行历史命令
evif history | grep "s3" | tail -1 | awk '{print $2}' | xargs evif
```

### 自动补全

```bash
# 启用 bash 自动补全
source <(evif completion bash)

# 启用 zsh 自动补全
source <(evif completion zsh)
```

## 故障排查

### 调试模式

```bash
# 启用详细输出
evif ls /local/ --verbose

# 启用调试日志
RUST_LOG=debug evif cat /local/file.txt
```

### 连接测试

```bash
# 测试服务器连接
evif health

# 测试特定插件
evif health /s3

# 查看服务器信息
evif server-info
```

### 查看日志

```bash
# 查看 EVIF 服务器日志
tail -f /var/log/evif/evif.log

# 过滤错误日志
grep ERROR /var/log/evif/evif.log
```

---

**更多示例:**
- [S3操作示例](./s3-ops.md)
- [向量搜索示例](./vector-search.md)
- [批量操作示例](./batch-ops.md)
- [集成示例](./integration.md)

**相关文档:**
- [SKILL.md](../SKILL.md) - EVIF 主技能
- [evif-manage.md](../evif-manage.md) - 插件管理
