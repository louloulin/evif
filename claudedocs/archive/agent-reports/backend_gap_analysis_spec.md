# EVIF vs AGFS Backend功能差距分析规范

## 创建日期: 2026-02-09
## 任务ID: task-1770645854-51bf
## 阶段: Phase 6 - Backend功能分析与完善

---

## 1. 概述 (Summary)

深入分析EVIF和AGFS在backend功能上的差距，生成详细的功能对比清单和实施建议。该分析将作为Phase 6后续完善EVIF backend功能的基础。

---

## 2. 分析范围 (Scope)

### 2.1 包含的功能领域
- REST API功能对比
- CLI命令功能对比
- 插件系统对比
- 文件系统操作对比
- WASM插件支持对比
- Shell/脚本能力对比
- 其他backend功能对比

### 2.2 不包含的范围
- UI前端功能（已在Phase 1-5完成）
- 性能基准测试
- 代码质量分析
- 安全审计

---

## 3. 验收标准 (Acceptance Criteria)

### 3.1 GIVEN 当前的EVIF和AGFS代码库
### WHEN 进行深度功能对比分析时
### THEN 生成包含以下内容的完整报告：

#### 标准1: REST API功能对比
**GIVEN** EVIF的100+ REST端点和AGFS的API端点
**WHEN** 逐个对比每个API类别时
**THEN** 输出包含以下内容的对比表：
- EVIF独有功能列表
- AGFS独有功能列表
- 两者共同拥有功能列表
- 功能实现差异说明

**示例格式**:
| 功能类别 | EVIF状态 | AGFS状态 | 差距说明 |
|---------|---------|---------|---------|
| 文件handles | ✅ 已实现 | ✅ 已实现 | 功能相当 |
| 全局handle管理 | ❌ 未实现 | ✅ 已实现 | **P1差距** |
| 协作功能 | ✅ 已实现 | ❌ 未实现 | EVIF优势 |

#### 标准2: CLI命令功能对比
**GIVEN** EVIF的70+ CLI命令和AGFS的50+ Shell命令
**WHEN** 对比命令能力时
**THEN** 输出包含：
- 按类别分组的命令对比表
- Shell脚本能力对比（管道、变量、控制流）
- 独特命令功能列表

**示例格式**:
| 命令类别 | EVIF命令数 | AGFS命令数 | 差距分析 |
|---------|-----------|-----------|---------|
| 文件操作 | 15 | 12 | EVIF更多 |
| 文本处理 | 20 | 18 | 相当 |
| Shell特性 | 0 | 完整支持 | **P1差距** |

#### 标准3: 插件系统对比
**GIVEN** EVIF的31个插件和AGFS的15个插件
**WHEN** 分析插件架构时
**THEN** 输出：
- 插件加载机制对比（动态.so vs WASM）
- 独有插件列表
- 插件能力差异（如VectorFS语义搜索）

**示例格式**:
| 插件特性 | EVIF | AGFS | 差距 |
|---------|------|------|------|
| 插件数量 | 31 | 15 | EVIF更多 |
| 动态.so加载 | ❌ | ✅ | **P2差距** |
| WASM支持 | ✅ (Extism) | ✅ (Wazero) | 实现不同 |
| 向量搜索插件 | ✅ vectorfs | ✅ vectorfs | 相当 |

#### 标准4: 文件系统操作对比
**GIVEN** 两者的VFS层和FUSE支持
**WHEN** 对比文件操作能力时
**THEN** 输出：
- VFS架构对比
- FUSE支持状态
- 流式文件操作支持

**示例格式**:
| 功能 | EVIF | AGFS | 差距 |
|------|------|------|------|
| VFS层 | ✅ 完整 | ✅ 完整 | 相当 |
| FUSE支持 | ✅ | ✅ | 相当 |
| 流式读取 | ✅ StreamReader | ✅ Streamer接口 | 相当 |

#### 标准5: Shell/脚本能力对比
**GIVEN** AGFS的全功能Python Shell和EVIF的CLI REPL
**WHEN** 对比脚本能力时
**THEN** 输出：
- Shell功能完整度对比
- 脚本语言支持对比
- 管道和重定向支持

**示例格式**:
| Shell特性 | EVIF CLI | AGFS Shell | 差距 |
|----------|----------|-----------|------|
| 管道 | ❌ | ✅ \| | **P2差距** |
| 变量 | 基础 | 完整 | **P2差距** |
| 控制流 | ❌ | ✅ if/for | **P2差距** |
| 函数 | ❌ | ✅ | **P3差距** |

#### 标准6: 优先级分类
**GIVEN** 所有识别的功能差距
**WHEN** 按影响程度分类时
**THEN** 为每个差距分配优先级：
- **P1**: 生产关键功能（如全局handle管理）
- **P2**: 重要增强功能（如Shell脚本）
- **P3**: 可选优化功能（如高级Shell特性）

**优先级判断标准**:
- P1: 严重影响生产使用或安全
- P2: 显著提升用户体验
- P3: 锦上添花的功能

#### 标准7: 实施建议
**GIVEN** 优先级分类后的差距清单
**WHEN** 生成实施建议时
**THEN** 为每个P1和P2差距提供：
- 技术实现方案
- 预计工作量（人天）
- 依赖关系
- 风险评估

**示例格式**:
### 差距: 全局handle管理
- **优先级**: P1
- **当前状态**: EVIF有handle API但无全局管理
- **目标**: 实现类似AGFS的全局handle表
- **技术方案**: 在evif-core中添加GlobalHandleManager
- **预计工作量**: 3-4天
- **依赖**: 无
- **风险**: 中等

---

## 4. 输入数据 (Input Data)

### 4.1 EVIF数据源
**代码位置**:
- REST API: `crates/evif-rest/src/routes.rs` (100+ 端点)
- CLI命令: `crates/evif-cli/src/cli.rs` (70+ 命令)
- 插件系统: `crates/evif-core/src/plugin.rs` (31插件)
- VFS层: `crates/evif-vfs/src/lib.rs`
- FUSE: `crates/evif-fuse/src/lib.rs`
- WASM: `crates/evif-rest/src/wasm_handlers.rs`

**已验证指标**:
- 42,505 LOC
- 68 CLI命令
- 66 REST routes
- 31 plugins
- 92-95% 完成度

### 4.2 AGFS数据源
**代码位置**:
- REST API: `agfs-server/pkg/handlers/handlers.go`
- Shell: `agfs-shell/agfs_shell/shell.py` (50+ 命令)
- 插件系统: `agfs-server/pkg/plugin/plugin.go` (15插件)
- 文件系统: `agfs-server/pkg/filesystem/filesystem.go`
- FUSE: `agfs-fuse/pkg/fusefs/fs.go`
- WASM: `agfs-server/pkg/plugin/api/wasm_plugin.go`

**已知特性**:
- Go backend (sync vs EVIF的async Rust)
- 全功能Shell with 管道、变量、控制流
- WASM instance pool
- VectorFS语义搜索

---

## 5. 输出格式 (Output Format)

### 5.1 报告结构
```markdown
# EVIF vs AGFS Backend功能差距分析报告

## 执行摘要
- 总体差距数量
- P1/P2/P3分类统计
- 关键发现

## 1. REST API功能对比
### 1.1 API端点对比表
### 1.2 EVIF独有功能
### 1.3 AGFS独有功能
### 1.4 功能实现差异

## 2. CLI/Shell功能对比
### 2.1 命令数量对比
### 2.2 Shell脚本能力对比
### 2.3 独特命令功能

## 3. 插件系统对比
### 3.1 插件数量对比
### 3.2 加载机制对比
### 3.3 独有插件列表

## 4. 文件系统操作对比
### 4.1 VFS架构对比
### 4.2 特殊功能对比

## 5. 优先级分类清单
### 5.1 P1差距（生产关键）
### 5.2 P2差距（重要增强）
### 5.3 P3差距（可选优化）

## 6. 实施建议
### 6.1 P1功能实施路线图
### 6.2 P2功能实施建议
### 6.3 依赖关系图

## 7. 总结与建议
- EVIF优势总结
- 关键差距总结
- 优先实施建议
```

### 5.2 对比表格式
每个功能领域使用统一的对比表格式：

```markdown
### 功能领域名称

| 功能 | EVIF | AGFS | 差距 | 优先级 |
|------|------|------|------|--------|
| 功能1 | ✅ 已实现 | ❌ 未实现 | EVIF优势 | - |
| 功能2 | ❌ 未实现 | ✅ 已实现 | 需实现 | P1 |
| 功能3 | ✅ 部分实现 | ✅ 完整实现 | 需完善 | P2 |
```

---

## 6. 边界情况 (Edge Cases)

### 6.1 功能等价性判断
- **问题**: 相似功能但实现不同如何判断
- **解决**: 基于用户能力而非实现细节
- **示例**: EVIF的Extism WASM vs AGFS的Wazero WASM → 功能等价

### 6.2 部分实现情况
- **问题**: 功能存在但不完整
- **解决**: 标记为"部分实现"并说明差距
- **示例**: EVIF有handle API但无全局管理

### 6.3 不同架构导致的差异
- **问题**: async vs sync架构差异
- **解决**: 关注功能而非实现
- **示例**: EVIF async Rust vs AGFS sync Go → 不影响功能对比

---

## 7. 非功能需求 (Non-functional Requirements)

### 7.1 分析质量要求
- **准确性**: 所有功能声明必须通过代码验证
- **完整性**: 覆盖所有主要backend功能
- **可验证性**: 每个差距必须有代码位置引用

### 7.2 报告质量要求
- **清晰性**: 使用表格和列表提高可读性
- **可操作性**: 实施建议具体且可执行
- **优先级明确**: P1/P2/P3分类理由充分

### 7.3 客观性要求
- **无偏见**: 承认EVIF的优势（如更多插件）
- **基于事实**: 所有判断基于代码证据
- **避免推测**: 不实现的功能不推测未来计划

---

## 8. 代码位置引用规范

### 8.1 EVIF代码引用格式
```
EVIF: <文件路径>:<行号>
示例: EVIF: crates/evif-rest/src/routes.rs:123-145
```

### 8.2 AGFS代码引用格式
```
AGFS: <文件路径>:<行号>
示例: AGFS: agfs-server/pkg/handlers/handlers.go:67-89
```

### 8.3 功能证据要求
每个功能声明必须包含：
- EVIF功能: 文件路径 + 函数名
- AGFS功能: 文件路径 + 函数名
- 差距声明: 双方代码位置

---

## 9. 分析方法论

### 9.1 功能发现方法
1. **静态代码分析**: 阅读routes.rs、handlers.go等核心文件
2. **文档参考**: 查阅API文档（agfs-server/api.md）
3. **测试验证**: 运行CLI命令验证功能
4. **配置文件分析**: 检查示例配置

### 9.2 对比分析方法
1. **建立分类体系**: 按功能领域分组
2. **逐项对比**: 使用GIVEN-WHEN-THEN格式
3. **优先级评估**: 基于生产影响程度
4. **实施建议**: 提供具体技术方案

### 9.3 质量保证
- **交叉验证**: 使用多个数据源验证功能
- **同行审查**: Spec Critic审查分析完整性
- **实施验证**: Implementer执行代码分析验证

---

## 10. 成功标准

### 10.1 分析完整性
- ✅ 覆盖所有主要backend功能领域
- ✅ 每个差距有明确的优先级
- ✅ 每个P1/P2差距有实施建议

### 10.2 报告质量
- ✅ 结构清晰，易于阅读
- ✅ 所有声明有代码证据
- ✅ 实施建议具体可操作

### 10.3 可用性
- ✅ 报告可直接用于Phase 6实施规划
- ✅ 优先级分类合理且可辩护
- ✅ 工作量估算现实可靠

---

## 附录A: 已知背景信息

### A.1 已有的分析记忆
- **mem-1770542885-d20c**: EVIF完成度92-95%，CLI和REST已超越AGFS
- **mem-1770542839-1bca**: EVIF实际完成度高于报告值
- **mem-1770542021-3c40**: EVIF架构更优（async vs sync），缺失P1全局handle管理

### A.2 已知差距（来自记忆）
1. **P1**: 全局handle管理（AGFS有，EVIF缺失）
2. **P2**: 动态.so加载（AGFS有，EVIF仅有WASM）
3. **P2**: Shell变量和脚本（AGFS完整支持，EVIF缺失）
4. **P3**: Shell高级特性（函数、递归等）

### A.3 已知EVIF优势
1. 更多插件（31 vs 15）
2. 更多CLI命令（68 vs 50+）
3. 异步架构（Tokio vs sync Go）
4. 更多REST端点（66+ vs AGFS基础端点）
5. 协作功能（AGFS未发现）

---

## 附录B: 参考资料

### B.1 EVIF文档
- REST API: `evif-web/` 中的API调用
- CLI使用: `crates/evif-cli/README.md`（如果存在）
- 插件开发: `crates/evif-core/src/plugin.rs` trait定义

### B.2 AGFS文档
- REST API: `agfs-server/api.md`
- Shell使用: `agfs-shell/agfs_shell/shell.py`
- 插件开发: `agfs-server/pkg/plugin/plugin.go`

### B.3 技术栈对比
| 维度 | EVIF | AGFS |
|------|------|------|
| 语言 | Rust | Go |
| 并发模型 | Async (Tokio) | Sync (goroutines) |
| WASM运行时 | Extism | Wazero |
| FUSE库 | fuser | go-fuse/v2 |
| 路由算法 | Radix Tree | Radix Tree |
| 插件数量 | 31 | 15 |

---

**规范编写者**: Spec Writer (📋)
**规范状态**: ✅ 完成 (spec.done)
**下一步**: 提交给Spec Critic审查
