# 完善EVIF Backend缺失功能实施规范

## 规范信息
- **任务ID**: task-1770645876-558c
- **优先级**: P2 (重要增强)
- **创建日期**: 2026-02-09
- **规范编写者**: 📋 Spec Writer
- **参考文档**: `.ralph/agent/backend_gap_analysis_report.md`

---

## 摘要

本规范定义了完善EVIF backend缺失功能的详细实施步骤。基于backend差距分析报告，需要实现4个关键功能：
- **P1**: 全局Handle管理 (生产关键)
- **P2-1**: Shell脚本能力 (重要增强)
- **P2-2**: 动态.so加载 (重要增强)
- **P2-3**: WASM Instance Pool (重要增强)

本规范采用分阶段实施策略，优先完成P1功能，然后根据资源和时间逐步实施P2功能。

---

## 验收标准 (Given-When-Then)

### P1功能: 全局Handle管理

#### AC-P1-001: 全局Handle管理器存在
**GIVEN** EVIF backend启动时
**WHEN** 初始化全局状态时
**THEN** 创建一个`GlobalHandleManager`实例，该实例:
  - 使用`Arc<RwLock<HashMap<u64, Handle>>>`存储handles
  - 使用`Arc<AtomicU64>`生成唯一handle ID
  - 注册到`AppState`中供所有handler访问

#### AC-P1-002: Handle注册和获取
**GIVEN** 一个打开的文件Handle
**WHEN** 调用`GlobalHandleManager::register(handle)`
**THEN**:
  - 返回一个唯一的u64类型handle ID
  - Handle存储到全局HashMap中
  - ID为原子递增，保证线程安全

**GIVEN** 一个已注册的handle ID
**WHEN** 调用`GlobalHandleManager::get(id)`
**THEN**:
  - 返回`Some(Handle)`如果ID存在
  - 返回`None`如果ID不存在或已过期

#### AC-P1-003: Handle生命周期管理
**GIVEN** 一个已注册的handle
**WHEN** 调用`GlobalHandleManager::close(id)`
**THEN**:
  - 从HashMap中移除handle
  - 调用handle的底层close方法
  - 返回`Ok(())`如果成功
  - 返回`Err(Error)`如果ID不存在

**GIVEN** 全局handle管理器
**WHEN** 调用`cleanup_expired(ttl)`方法
**THEN**:
  - 移除所有超过TTL时间未使用的handle
  - 防止handle泄漏
  - 默认TTL为30分钟

#### AC-P1-004: REST API集成
**GIVEN** EVIF REST服务运行中
**WHEN** 客户端调用以下API时
**THEN** API使用全局handle管理器:

1. `GET /api/v1/handles`
   - 返回所有活跃handles的列表
   - 每个handle包含: id, path, created_at, last_accessed
   - 响应格式:
     ```json
     {
       "handles": [
         {"id": 123, "path": "/local/file.txt", "created_at": "2026-02-09T10:00:00Z", "last_accessed": "2026-02-09T10:05:00Z"}
       ]
     }
     ```

2. `DELETE /api/v1/handles/:id`
   - 关闭指定handle
   - 成功返回204 No Content
   - handle不存在返回404 Not Found

3. `POST /api/v1/files/{path}/open` (修改)
   - 使用全局handle管理器注册handle
   - 返回的handle ID在全局管理器中持久化

#### AC-P1-005: 线程安全
**GIVEN** 多个并发请求
**WHEN** 同时访问全局handle管理器时
**THEN**:
  - 所有操作保证线程安全
  - 使用`RwLock`允许多读或单写
  - 无数据竞争 (data races)
  - 无deadlock

#### AC-P1-006: 监控和调试
**GIVEN** 全局handle管理器运行中
**WHEN** 需要监控handle状态时
**THEN**:
  - 提供`stats()`方法返回统计信息
  - 统计信息包括: 总handle数、活跃handle数、过期handle数
  - 日志记录handle创建、关闭、清理事件

---

### P2-1功能: Shell脚本能力

#### AC-P2-001: Shell REPL基础
**GIVEN** 用户执行`evif shell`命令
**WHEN** Shell启动时
**THEN**:
  - 显示提示符 (例如 `evif> `)
  - 接受用户输入
  - 执行命令并显示结果
  - 支持Ctrl+C退出

#### AC-P2-002: 管道操作 (Pipe)
**GIVEN** 两个或多个EVIF命令
**WHEN** 使用`|`管道操作符连接命令时
**THEN**:
  - 第一个命令的stdout成为第二个命令的stdin
  - 支持多级管道 (例如 `cmd1 | cmd2 | cmd3`)
  - 每个命令在独立子进程中执行
  - 正确处理退出码

**示例**:
```bash
evif> ls /local | grep "\.txt$" | wc -l
```

#### AC-P2-003: 变量系统
**GIVEN** Shell交互环境
**WHEN** 用户使用变量时
**THEN**:

1. **变量赋值**:
   ```bash
   evif> MY_PATH=/local/data
   ```

2. **变量引用**:
   ```bash
   evif> ls $MY_PATH
   evif> echo ${MY_PATH}
   ```

3. **环境变量**:
   - 继承父进程环境变量
   - `export`命令导出变量到子进程
   - `unset`命令删除变量

4. **内置变量**:
   - `$HOME`: EVIF主目录
   - `$PWD`: 当前工作目录
   - `$?`: 上一个命令的退出码

#### AC-P2-004: 控制流
**GIVEN** Shell脚本环境
**WHEN** 用户使用控制流语句时
**THEN**:

1. **if条件语句**:
   ```bash
   if [ -f "/local/file.txt" ]; then
       cat /local/file.txt
   fi
   ```

2. **for循环**:
   ```bash
   for file in $(ls /local); do
       echo "Processing: $file"
   done
   ```

3. **while循环**:
   ```bash
   while read line; do
       echo "$line"
   done < /local/input.txt
   ```

#### AC-P2-005: 脚本执行
**GIVEN** 一个包含Shell命令的文本文件
**WHEN** 执行`source`或`.`命令时
**THEN**:
  - 逐行读取并执行文件中的命令
  - 保持当前Shell环境 (变量、工作目录)
  - 支持shebang行 (`#!/usr/bin/env evif-shell`)
  - 错误时报告文件名和行号

**示例**:
```bash
evif> source script.evfs
# 或
evif> . script.evfs
```

---

### P2-2功能: 动态.so加载

#### AC-P2-006: Native Plugin Trait定义
**GIVEN** EVIF plugin系统
**WHEN** 定义Native Plugin接口时
**THEN**:

```rust
pub trait NativePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn filesystem(self) -> Box<dyn FileSystem>;
}

extern "C" fn register() -> *mut dyn NativePlugin;
```

#### AC-P2-007: 动态加载器实现
**GIVEN** 一个编译好的.so插件文件
**WHEN** 调用`NativePluginLoader::load(path)`
**THEN**:
  - 使用`libloading` crate加载.so文件
  - 调用`register`符号获取插件实例
  - 验证插件ABI兼容性
  - 返回`Box<dyn FileSystem>`或错误
  - 安全处理unsafe FFI调用

#### AC-P2-008: 插件API集成
**GIVEN** EVIF REST API
**WHEN** 管理native插件时
**THEN**:

1. `POST /api/v1/plugins/native/load`
   - 请求体:
     ```json
     {"path": "/path/to/plugin.so"}
     ```
   - 成功返回201 Created
   - 失败返回400/500 with error details

2. `GET /api/v1/plugins/native`
   - 返回已加载的native插件列表

3. `DELETE /api/v1/plugins/native/:name`
   - 卸载指定的native插件

#### AC-P2-009: 跨平台支持
**GIVEN** EVIF运行在不同操作系统
**WHEN** 加载native插件时
**THEN**:
  - Linux: 加载`.so`文件
  - macOS: 加载`.dylib`文件
  - Windows: 加载`.dll`文件
  - 编译时自动检测平台扩展名

---

### P2-3功能: WASM Instance Pool

#### AC-P2-010: Instance Pool结构
**GIVEN** EVIF WASM插件系统
**WHEN** 初始化WASM pool时
**THEN**:
  - 创建固定大小的instance池 (默认10个)
  - 使用`mpsc::channel`实现instance请求队列
  - 每个instance包装为`PooledPlugin`结构
  - 配置可调 (通过环境变量或配置文件)

#### AC-P2-011: Instance获取和释放
**GIVEN** 一个运行中的WASM pool
**WHEN** handler请求instance时
**THEN**:
  - 调用`pool.acquire().await`
  - 如果有空闲instance，立即返回
  - 如果池满，等待直到有instance释放
  - 返回的`PooledPlugin`自动实现Drop trait归还instance

**示例**:
```rust
let plugin = pool.acquire().await?;
let result = plugin.call("function", data).await?;
// plugin离开作用域自动归还
```

#### AC-P2-012: Instance状态管理
**GIVEN** 一个被归还的WASM instance
**WHEN** instance返回池中时
**THEN**:
  - 重置instance状态 (清除内存、globals)
  - 验证instance仍然可用
  - 如果instance损坏，从池中移除并创建新instance
  - 记录instance创建/销毁统计

#### AC-P2-013: Pool大小和清理
**GIVEN** WASM instance pool
**WHEN** 管理pool资源时
**THEN**:
  - 支持动态调整pool大小
  - 空闲instance超时自动销毁 (默认5分钟)
  - 提供`pool.stats()`返回:
    - 当前instance数
    - 活跃instance数
    - 等待队列长度
    - 总请求数
  - 关闭时优雅清理所有instance

---

## 输入/输出示例

### P1: 全局Handle管理

#### 输入示例
```http
POST /api/v1/files/local/test.txt/open
```

#### 输出示例
```json
{
  "handle_id": 12345,
  "path": "/local/test.txt",
  "mode": "read",
  "created_at": "2026-02-09T10:00:00Z"
}
```

### P2-1: Shell脚本

#### 输入示例 (script.evfs)
```bash
#!/usr/bin/env evif-shell
# 备份脚本示例

SRC_DIR=/local/data
BACKUP_DIR=/backup

for file in $(ls $SRC_DIR); do
    if [ -f "$SRC_DIR/$file" ]; then
        cp "$SRC_DIR/$file" "$BACKUP_DIR/$file.bak"
        echo "Backed up: $file"
    fi
done
```

#### 执行输出
```bash
evif> source backup.evfs
Backed up: document.txt
Backed up: image.png
Backed up: data.json
```

### P2-2: 动态.so插件

#### 示例插件 (plugin.c)
```c
#include <evif-plugin.h>

evif_fs_t* register() {
    static evif_fs_t fs = {
        .name = "custom-fs",
        .version = "1.0.0",
        .read = custom_read,
        .write = custom_write,
        // ...
    };
    return &fs;
}
```

#### 加载请求
```http
POST /api/v1/plugins/native/load
{
  "path": "/usr/local/lib/evif/plugins/custom-fs.so"
}
```

### P2-3: WASM Pool统计

#### 输入
```http
GET /api/v1/plugins/wasm/pool/stats
```

#### 输出
```json
{
  "pool_size": 10,
  "active_instances": 7,
  "idle_instances": 3,
  "waiting_requests": 0,
  "total_requests": 1234,
  "avg_wait_time_ms": 5.2
}
```

---

## 边界情况和错误处理

### P1边界情况

#### BC-P1-001: Handle ID冲突
**场景**: `AtomicU64`溢出回绕
**处理**: 检测回绕，panic或返回错误 (理论上不可能，2^64次handle创建)

#### BC-P1-002: Handle已关闭
**场景**: 尝试关闭已关闭的handle
**处理**: 返回`Err(Error::HandleClosed)` 而非panic

#### BC-P1-003: 并发关闭同一handle
**场景**: 两个请求同时关闭同一handle
**处理**: 使用`RwLock`确保原子性，第二次关闭返回`Err(Error::HandleNotFound)`

#### BC-P1-004: TTL清理时访问handle
**场景**: cleanup正在遍历handles，同时有请求访问handle
**处理**: `RwLock`读锁允许并发访问，写锁独占清理

### P2-1边界情况

#### BC-P2-001: 管道命令失败
**场景**: 管道中间命令失败
**处理**: 终止整个管道，返回失败命令的退出码

#### BC-P2-002: 循环无限
**场景**: while条件永真
**处理**: Ctrl+C中断，或最大迭代次数限制 (默认10000)

#### BC-P2-003: 变量未定义
**场景**: 引用未定义的变量
**处理**: 展开为空字符串，显示警告

#### BC-P2-004: 脚本文件不存在
**场景**: `source`指定不存在的文件
**处理**: 返回`Err(Error::FileNotFound(path))`

### P2-2边界情况

#### BC-P2-005: .so文件不存在
**场景**: 加载路径不存在的.so
**处理**: 返回`Err(Error::PluginNotFound(path))`

#### BC-P2-006: ABI不兼容
**场景**: .so编译时使用的EVIF版本不同
**处理**: 版本检查，返回`Err(Error::AbiMismatch)`

#### BC-P2-007: 符号缺失
**场景**: .so缺少`register`函数
**处理**: 捕获`libloading::Error::SymbolNotFound`

#### BC-P2-008: 插件panic
**场景**: native插件代码panic
**处理**: 使用`catch_unwind`捕获，卸载插件，返回500错误

### P2-3边界情况

#### BC-P2-009: Pool满
**场景**: 所有instance都在使用，新请求到来
**处理**: 请求排队等待，超时返回`Err(Error::PoolTimeout)` (默认30秒)

#### BC-P2-010: Instance创建失败
**场景**: WASM文件损坏或内存不足
**处理**: 返回错误，从池中移除损坏instance，尝试创建新instance

#### BC-P2-011: Instance永不释放
**场景**: handler忘记释放instance
**处理**: `PooledPlugin`实现Drop，RAII确保释放

#### BC-P2-012: Pool关闭时还有活跃instance
**场景**: 服务器关闭，但instance还在使用
**处理**: 等待所有instance归还 (超时10秒)，强制关闭

---

## 非功能需求

### NFR-001: 性能
- **P1 Handle管理**: handle注册/获取 < 1μs
- **P2-1 Shell**: 命令启动 < 100ms
- **P2-2 .so加载**: 插件加载 < 500ms
- **P2-3 WASM pool**: instance获取 < 10ms (90th percentile)

### NFR-002: 可扩展性
- **P1**: 支持10000+并发handles
- **P2-3**: Pool大小可配置1-1000 instances

### NFR-003: 可靠性
- **P1**: Handle清理机制防止泄漏
- **P2-3**: Instance健康检查，自动替换损坏instance

### NFR-004: 安全性
- **P2-2**: .so插件沙箱隔离 (可选)
- **P2-2**: 仅加载指定目录的.so (防止路径遍历)
- **P1**: Handle权限验证

### NFR-005: 可维护性
- 所有新代码添加单元测试 (覆盖率 >80%)
- 所有新API添加文档注释
- 关键路径添加集成测试

---

## 超出范围

以下功能不在本规范范围内:

1. **P3功能**: Shell高级特性 (函数定义、别名系统、命令历史增强)
2. **UI改造**: Shell Web界面 (仅CLI实现)
3. **插件开发工具**: .so插件开发脚手架
4. **性能优化**: WASM JIT编译 (当前使用解释器)
5. **监控增强**: Prometheus metrics导出 (已有基础metrics)
6. **认证授权**: 多用户Shell权限管理
7. **远程Shell**: SSH服务器实现

---

## 实施计划

### 阶段1: P1全局Handle管理 (3-4天)

**Day 1: 核心实现**
- 创建`crates/evif-core/src/handle.rs`
- 实现`GlobalHandleManager`结构
- 实现注册/获取/关闭方法
- 单元测试

**Day 2: REST API集成**
- 修改`crates/evif-rest/src/handle_handlers.rs`
- 添加`GET /api/v1/handles`
- 添加`DELETE /api/v1/handles/:id`
- 集成测试

**Day 3: TTL和清理**
- 实现handle TTL机制
- 实现定时清理任务
- 添加监控和日志
- 文档编写

**Day 4: 测试和调优**
- 压力测试 (1000+并发handles)
- 线程安全验证
- 性能benchmark
- Code review

### 阶段2: P2功能 (可选，根据资源)

#### P2-3: WASM Instance Pool (2天)
- **Day 1**: Pool结构实现、acquire/return逻辑
- **Day 2**: 集成到wasm_handlers、性能测试

#### P2-2: 动态.so加载 (2-3天)
- **Day 1**: NativePlugin trait设计、libloading集成
- **Day 2**: API实现、跨平台支持
- **Day 3**: 示例插件、文档

#### P2-1: Shell脚本能力 (5-7天)
- **Day 1-2**: Lexer和Parser实现
- **Day 3-4**: Interpreter核心
- **Day 5-6**: 管道、变量、控制流
- **Day 7**: 测试和文档

---

## 依赖关系

```
P1 (全局Handle管理)
├── 无依赖
└── 可立即开始

P2-3 (WASM Pool)
├── 依赖现有WASM系统
└── 与P1独立

P2-2 (动态.so)
├── 依赖plugin系统
└── 与P1、P2-3独立

P2-1 (Shell脚本)
├── 依赖CLI命令系统
└── 与其他功能独立
```

**并行实施建议**:
- Sprint 1: P1 (必须完成)
- Sprint 2: P2-3 (低风险，高价值)
- Sprint 3: P2-2 或 P2-1 (根据需求优先级)

---

## 验证清单

实施完成后，验证以下项目:

### P1验证
- [ ] 全局handle管理器编译通过
- [ ] 单元测试覆盖率 >80%
- [ ] `GET /api/v1/handles`返回正确数据
- [ ] `DELETE /api/v1/handles/:id`成功关闭handle
- [ ] 并发测试无data race
- [ ] TTL清理机制工作正常
- [ ] 文档完整 (API文档、架构文档)

### P2-1验证
- [ ] `evif shell`命令启动REPL
- [ ] 管道操作符 `|` 工作正常
- [ ] 变量赋值和引用正确
- [ ] if/for/while控制流执行正确
- [ ] `source`命令执行脚本
- [ ] 错误处理和报告清晰

### P2-2验证
- [ ] `.so`文件成功加载
- [ ] Native插件正确实现FileSystem trait
- [ ] REST API创建/删除native插件
- [ ] 跨平台兼容 (Linux/macOS/Windows)
- [ ] ABI不兼容时返回清晰错误

### P2-3验证
- [ ] WASM pool正确初始化
- [ ] Instance获取/释放无泄漏
- [ ] Pool统计API返回正确数据
- [ ] 高负载下性能提升 >30%
- [ ] Instance损坏自动替换

---

## 风险评估

### P1风险
**风险等级**: 中等
- **风险**: Handle生命周期管理复杂，可能泄漏
- **缓解**: TTL自动清理、单元测试、压力测试
- **应急方案**: 如果问题严重，可以禁用全局管理，回退到请求级handles

### P2-1风险
**风险等级**: 高
- **风险**: Shell语言复杂，边界情况多
- **缓解**: 参考AGFS实现、充分测试、渐进式实现
- **应急方案**: 仅实现基础管道和变量，控制流可选

### P2-2风险
**风险等级**: 中等
- **风险**: Unsafe FFI调用可能导致崩溃
- **缓解**: 使用`catch_unwind`、沙箱隔离、严格ABI检查
- **应急方案**: 仅在稳定平台支持 (.so on Linux)

### P2-3风险
**风险等级**: 低
- **风险**: Instance状态管理可能有bug
- **缓解**: 健康检查、自动替换、详细日志
- **应急方案**: 禁用pool，回退到每次创建新instance

---

## 成功标准

本规范实施完成的标准:

1. **P1功能**:
   - 所有AC-P1-*验收标准通过
   - 单元测试和集成测试全部通过
   - 性能基准达标
   - 文档完整

2. **P2功能** (可选):
   - 实施的P2功能对应验收标准通过
   - 测试覆盖率 >80%
   - 性能无明显退化
   - 代码审查通过

3. **总体**:
   - CI/CD pipeline通过
   - 无已知critical/high severity bug
   -向后兼容性保持 (不破坏现有API)

---

## 附录

### A. 相关代码位置

**P1实现**:
- 新增: `crates/evif-core/src/handle.rs`
- 修改: `crates/evif-rest/src/handle_handlers.rs`
- 修改: `crates/evif-rest/src/routes.rs`
- 修改: `crates/evif-rest/src/handlers.rs`

**P2-1实现**:
- 新增: `crates/evif-shell/`
- 修改: `crates/evif-cli/src/shell.rs`

**P2-2实现**:
- 修改: `crates/evif-core/src/plugin.rs`
- 新增: `crates/evif-plugins/native/loader.rs`

**P2-3实现**:
- 新增: `crates/evif-rest/src/wasm_pool.rs`
- 修改: `crates/evif-rest/src/wasm_handlers.rs`

### B. 参考资料

- AGFS Global Handle实现: `agfs-server/pkg/handlers/handlers.go`
- AGFS Shell实现: `agfs-shell/agfs_shell/shell.py`
- Rust libloading文档: https://docs.rs/libloading/
- Extism WASM文档: https://extism.org/docs

---

**规范版本**: 1.0
**最后更新**: 2026-02-09
**状态**: ✅ Ready for Review
