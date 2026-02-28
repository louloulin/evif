# EVIF CLI 运行模式说明（Phase 10.2）

本文档说明 evif-cli 的运行模式与能力范围，便于用户与集成方预期行为。

---

## 一、运行模式

### 1.1 单命令模式（非 REPL）

直接执行子命令时，每次调用仅执行**一条命令**，无会话状态、无管道、无变量：

```bash
evif ls /
evif cat /mem/foo.txt
evif --server http://localhost:8081 run
```

- `evif <command> [args]`：单次执行后退出。
- 不支持：管道（`|`）、重定向（`>`、`>>`）、环境变量替换（`$VAR`）、多命令串联（`;`）。

### 1.2 REPL 模式（交互式）

使用 `evif run`（或配置为默认）进入**交互式 REPL**：

- 提示符形式：`evif <server>>`，如 `evif http://localhost:8081>`。
- 支持命令：`help`、`exit`/`quit`、`ls`、`cat`、`write`、`mkdir`、`rm`、`mv`、`cp`、`stat`、`grep`、`digest`/`checksum`、`mount`、`umount`、`mounts` 等（以 `help` 输出为准）。
- 输入 `help` 可查看当前支持的命令列表与用法。

---

## 二、管道与重定向（当前范围）

### 2.1 管道（|）

- **REPL 内**：支持**简化管道**——按 `|` 分割后**顺序执行**各段命令，前一段的输出**未**作为后一段的输入（即非标准 Shell 管道语义）。
- **单命令模式**：不支持管道。

### 2.2 重定向

- **输出重定向**（`>`、`>>`）：当前**未实现**，重定向符号会被当作参数或忽略。
- **输入重定向**（`<`）：当前**未实现**。

### 2.3 变量与控制流

- **变量替换**（如 `$path`、`${var}`）：**不支持**。
- **控制流**（如 `if`/`else`、`for`、`while`）：**不支持**。
- **脚本文件**：可通过 `evif script <file>` 等方式执行脚本内多行命令（若已实现），但仍为单条/顺序执行，无完整 Shell 语法。

---

## 三、与 AGFS Shell 的差异

| 能力         | AGFS Shell     | EVIF CLI（当前）     |
|--------------|----------------|----------------------|
| 命令数量     | 54 个          | 约 27 个（含 MountPlugin/UnmountPlugin 动态挂载/卸载；Ls/Cat/Write/Mkdir/Rm/Mv/Stat/Touch/Health/Grep/Digest/Head/Tail/Tree/Cp/Stats/Mount/Umount/ListMounts 等） |
| 管道         | 完整管道语义   | REPL 内简化（顺序执行） |
| 重定向       | 支持           | 未实现               |
| 变量/控制流  | 支持           | 未支持               |
| 脚本         | 支持           | 部分/简化            |

EVIF CLI 定位为**单命令 + 可选 REPL**，满足对 EVIF 服务的常用文件/挂载操作；完整 Shell 能力（管道、重定向、变量、控制流）为可选后续增强。

---

## 四、建议用法

- **自动化/脚本**：使用单命令调用（如 `evif ls /mem`），或通过 HTTP 直接调用 evif-rest API / evif-mcp 工具。
- **交互式探索**：使用 `evif run` 进入 REPL，输入 `help` 查看命令。
- **复杂流水线**：当前需在外部脚本中多次调用 `evif` 或调用 REST/MCP，在脚本内拼接输入输出。

---

**文档版本**：与 EVIF 2.4 Phase 10.2 对应；明确当前 CLI 为“单命令 + REPL”、简化管道、无重定向与变量。
