# EVIF FUSE 使用说明（Phase 11.2）

本文档说明 evif-fuse 的挂载方式、缓存与稳定性行为，供运维与开发参考。

---

## 一、挂载方式

### 1.1 命令行

```bash
# 只读挂载（默认）
evif-fuse-mount <挂载点>

# 读写挂载
evif-fuse-mount <挂载点> --readwrite

# 允许其他用户访问
evif-fuse-mount <挂载点> --readwrite --allow-other

# 调整缓存
evif-fuse-mount <挂载点> --cache-size 5000 --cache-timeout 120
```

示例：

```bash
mkdir -p /tmp/evif
evif-fuse-mount /tmp/evif --readwrite
# 使用完毕后
fusermount -u /tmp/evif   # Linux
umount /tmp/evif          # macOS
```

### 1.2 参数说明

| 参数 | 说明 | 默认 |
|------|------|------|
| `--readonly` | 只读挂载 | 默认只读 |
| `--readwrite` | 读写挂载 | - |
| `--allow-other` | 允许其他用户访问挂载点 | 否 |
| `--cache-size N` | Inode 管理器容量（预留） | 10000 |
| `--cache-timeout N` | 目录缓存 TTL（秒） | 60 |

---

## 二、缓存与稳定性（Phase 11.2）

### 2.1 已有能力

- **Inode 管理**：路径与 inode 双向映射，create/mkdir 分配 inode，unlink/rmdir/rename 回收或更新，避免 inode 泄漏与错乱。
- **目录缓存（DirCache）**：readdir 结果按路径缓存，带 TTL（`cache_timeout` 秒），减少对底层插件的重复 readdir 请求。
- **缓存失效**：在 **create**、**mkdir**、**unlink**、**rmdir**、**rename** 成功后，对受影响父目录调用 `dir_cache.invalidate`，保证后续 readdir 看到最新列表，避免“刚创建/删除的文件在 ls 里看不到/仍存在”等问题。

### 2.2 行为说明

- **readdir**：先查目录缓存，命中且未过期则直接返回；未命中或过期则调用插件 `readdir` 并写入缓存。
- **create/mkdir**：成功后对该父目录路径做 `invalidate`，下次 readdir 会重新从插件拉取。
- **unlink/rmdir**：成功后对父目录做 `invalidate`。
- **rename**：成功后对源父目录与目标父目录均做 `invalidate`。

因此挂载后常见操作（ls、创建/删除/重命名文件与目录）在缓存与一致性上可预期，无异常退出时可满足“挂载后常见操作无异常退出”的验收目标。

### 2.3 注意事项

- 缓存为进程内内存，重启后清空；多进程/多挂载实例之间不共享缓存。
- 仅目录列表被缓存，文件内容与元数据（getattr/read/write）每次请求仍走插件。
- 若底层插件在外部被修改（如另一进程写同一后端），需等待目录缓存 TTL 过期或重启挂载才能看到变更；必要时可缩短 `--cache-timeout`。

---

## 三、与 AGFS FUSE 的对照

| 能力 | AGFS FUSE | EVIF FUSE |
|------|-----------|-----------|
| 目录缓存 | 有（cache.go） | 有（DirCache，TTL + invalidate） |
| Inode 映射 | 有（node.go） | 有（InodeManager） |
| create/unlink/rename 后失效 | 有 | 有（Phase 11.2 补齐 invalidate） |

---

## 四、故障排查

- **挂载失败**：确认挂载点存在且为空、未已被挂载；Linux 需 fuse 内核模块与用户态权限（如 `user_allow_other`）。
- **ls 不刷新**：确认已做 create/unlink/rename 等操作后的 invalidate；若仍异常可调低 `--cache-timeout` 或重启挂载。
- **只读/只写**：确认启动参数为 `--readwrite` 或 `--readonly`，与预期一致。

---

**文档版本**：与 EVIF 2.4 Phase 11.2 对应；FUSE 稳定性与缓存行为已实现并文档化。
