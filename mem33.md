# EVIF Obsidian 插件开发计划

> 创建时间：2026-04-30
> 更新：2026-04-30
> 目标：设计 EVIF Obsidian 插件，将 EVIF 的能力（上下文分层、技能系统、记忆系统）集成到 Obsidian

---

## 一、核心概念

### 1.1 EVIF Obsidian 插件是什么？

**EVIF Obsidian 插件**是一个运行在 Obsidian 内部的插件，它：

1. 将 Obsidian 的笔记与 EVIF 后端同步
2. 暴露 EVIF 的核心能力到 Obsidian：
   - `/context/L0/L1/L2` 上下文分层
   - `/skills` 技能系统
   - `/memories` 记忆系统
   - `/queue` 任务队列
3. 允许 Obsidian 用户使用 AI Agent 的持久化能力

### 1.2 与 VaultSync 的区别

| 功能 | VaultSync | EVIF Obsidian 插件 |
|------|------------|-------------------|
| 同步目标 | Dropbox | EVIF 后端 |
| 基本同步 | ✅ | ✅ |
| 上下文分层 | ❌ | ✅ L0/L1/L2 |
| 技能执行 | ❌ | ✅ SKILL.md |
| 记忆系统 | ❌ | ✅ 向量记忆 |
| Agent 协作 | ❌ | ✅ PipeFS |
| MCP 集成 | ❌ | ✅ |

---

## 二、EVIF Obsidian 插件架构

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         Obsidian App                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │              EVIF Obsidian 插件 (TypeScript)               │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │  │
│  │  │ Context Panel │  │ Skill Runner │  │ Memory Panel │    │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘    │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │  │
│  │  │ Queue Viewer │  │  MCP Bridge   │  │ Sync Engine  │    │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘    │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                               │                                    │
│                    EVIF REST API / MCP Protocol                    │
│                               ▼                                    │
├─────────────────────────────────────────────────────────────────┤
│                         EVIF Backend                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ContextFS│  │ SkillFS │  │VectorFS │  │ QueueFS │           │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 插件组件

| 组件 | 功能 | 技术 |
|------|------|------|
| **Context Panel** | 显示 L0/L1/L2 上下文 | React/Svelte 面板 |
| **Skill Runner** | 执行 SKILL.md 技能 | API 调用 |
| **Memory Panel** | 向量记忆搜索 | 语义搜索 |
| **Queue Viewer** | 任务队列管理 | 实时更新 |
| **MCP Bridge** | 连接 Claude Desktop | MCP Protocol |
| **Sync Engine** | Obsidian ↔ EVIF 同步 | 文件系统监控 |

---

## 三、插件功能设计

### 3.1 P0 必须功能

| 功能 | 描述 | 实现方式 |
|------|------|----------|
| **EVIF 连接** | 配置 EVIF 服务器地址和认证 | 设置面板 |
| **笔记同步** | 将 Obsidian 笔记同步到 EVIF | 文件监听 |
| **上下文写入** | 将当前笔记写入 L0/L1/L2 | 命令面板 |
| **技能执行** | 在笔记中执行 SKILL.md | 侧边栏按钮 |
| **记忆检索** | 搜索相关记忆 | 命令面板 |

### 3.2 P1 重要功能

| 功能 | 描述 | 实现方式 |
|------|------|----------|
| **L0/L1/L2 视图** | 可视化上下文层级 | 面板视图 |
| **技能市场** | 浏览可用技能 | 侧边栏 |
| **记忆面板** | 显示相关记忆 | 右侧面板 |
| **任务队列** | 查看/管理任务 | 面板视图 |
| **Agent 通信** | PipeFS 消息传递 | 模态窗口 |

### 3.3 P2 增强功能

| 功能 | 描述 | 实现方式 |
|------|------|----------|
| **模板生成** | 基于上下文生成笔记模板 | AI API |
| **自动摘要** | 生成笔记摘要 | LLM API |
| **双向链接增强** | 结合向量记忆 | 图数据库 |
| **协作视图** | 多用户上下文共享 | 实时同步 |

---

## 四、技术实现

### 4.1 项目结构

```
evif-obsidian-plugin/
├── src/
│   ├── main.ts                 # 插件入口
│   ├── settings/
│   │   └── SettingsTab.ts      # 设置面板
│   ├── panels/
│   │   ├── ContextPanel.ts    # 上下文面板
│   │   ├── MemoryPanel.ts     # 记忆面板
│   │   ├── QueuePanel.ts      # 队列面板
│   │   └── SkillPanel.ts      # 技能面板
│   ├── commands/
│   │   ├── context.ts         # 上下文命令
│   │   ├── skill.ts           # 技能命令
│   │   ├── memory.ts          # 记忆命令
│   │   └── sync.ts            # 同步命令
│   ├── api/
│   │   ├── evif-client.ts     # EVIF API 客户端
│   │   └── mcp-bridge.ts      # MCP 协议桥接
│   └── sync/
│       └── sync-engine.ts      # 同步引擎
├── styles.css                   # 样式
├── manifest.json               # Obsidian 清单
└── package.json
```

### 4.2 API 客户端

```typescript
// src/api/evif-client.ts
export class EvifClient {
  private baseUrl: string;
  private apiKey: string;

  constructor(baseUrl: string, apiKey: string) {
    this.baseUrl = baseUrl;
    this.apiKey = apiKey;
  }

  // 上下文操作
  async readContext(layer: 'L0' | 'L1' | 'L2', path: string): Promise<string> {
    const response = await fetch(`${this.baseUrl}/context/${layer}/${path}`, {
      headers: { 'X-API-Key': this.apiKey }
    });
    return response.text();
  }

  async writeContext(layer: 'L0' | 'L1' | 'L2', path: string, content: string): Promise<void> {
    await fetch(`${this.baseUrl}/context/${layer}/${path}`, {
      method: 'PUT',
      headers: {
        'X-API-Key': this.apiKey,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ content })
    });
  }

  // 技能操作
  async listSkills(): Promise<Skill[]> {
    const response = await fetch(`${this.baseUrl}/skills`, {
      headers: { 'X-API-Key': this.apiKey }
    });
    return response.json();
  }

  async executeSkill(skillName: string, input: string): Promise<SkillResult> {
    const response = await fetch(`${this.baseUrl}/skills/${skillName}/execute`, {
      method: 'POST',
      headers: {
        'X-API-Key': this.apiKey,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ input })
    });
    return response.json();
  }

  // 记忆操作
  async searchMemories(query: string, limit: number = 10): Promise<Memory[]> {
    const response = await fetch(`${this.baseUrl}/memories/search`, {
      method: 'POST',
      headers: {
        'X-API-Key': this.apiKey,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ query, limit })
    });
    return response.json();
  }

  async memorize(content: string, tags: string[]): Promise<void> {
    await fetch(`${this.baseUrl}/memories`, {
      method: 'POST',
      headers: {
        'X-API-Key': this.apiKey,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ content, tags })
    });
  }
}
```

### 4.3 设置面板

```typescript
// src/settings/SettingsTab.ts
export class EvifSettingsTab extends PluginSettingTab {
  constructor(plugin: EvifPlugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  display(): void {
    const { containerEl } = this;
    containerEl.empty();
    containerEl.createEl('h2', { text: 'EVIF 设置' });

    new Setting(containerEl)
      .setName('EVIF 服务器地址')
      .setDesc('例如: http://localhost:8081')
      .addText(text => text
        .setValue(this.plugin.settings.evifUrl)
        .onChange(async (value) => {
          this.plugin.settings.evifUrl = value;
          await this.plugin.saveSettings();
        }));

    new Setting(containerEl)
      .setName('API 密钥')
      .setDesc('从 EVIF 服务器获取')
      .addText(text => text
        .setValue(this.plugin.settings.apiKey)
        .onChange(async (value) => {
          this.plugin.settings.apiKey = value;
          await this.plugin.saveSettings();
        }));

    new Setting(containerEl)
      .setName('自动同步')
      .setDesc('笔记更改时自动同步到 EVIF')
      .addToggle(toggle => toggle
        .setValue(this.plugin.settings.autoSync)
        .onChange(async (value) => {
          this.plugin.settings.autoSync = value;
          await this.plugin.saveSettings();
        }));

    new Setting(containerEl)
      .setName('同步间隔')
      .setDesc('自动同步间隔（秒）')
      .addText(text => text
        .setValue(String(this.plugin.settings.syncInterval))
        .onChange(async (value) => {
          this.plugin.settings.syncInterval = parseInt(value) || 30;
          await this.plugin.saveSettings();
        }));
  }
}
```

### 4.4 上下文面板

```typescript
// src/panels/ContextPanel.ts
export class ContextPanel extends ItemView {
  private evif: EvifClient;
  private vault: Vault;

  constructor(leaf: WorkspaceLeaf, evif: EvifClient) {
    super(leaf);
    this.evif = evif;
    this.vault = this.app.vault;
  }

  getViewType(): string {
    return 'evif-context-panel';
  }

  getIcon(): string {
    return 'brain';
  }

  async onOpen(): Promise<void> {
    const container = this.containerEl;
    container.empty();

    // L0 当前任务
    const l0Section = container.createDiv('context-section');
    l0Section.createEl('h3', { text: 'L0 - 当前任务' });
    const l0Content = l0Section.createDiv('context-content');
    l0Content.setText(await this.evif.readContext('L0', 'current'));

    // L1 决策记录
    const l1Section = container.createDiv('context-section');
    l1Section.createEl('h3', { text: 'L1 - 决策记录' });
    const l1List = l1Section.createEl('ul');
    const l1Decisions = await this.evif.readContext('L1', 'decisions.md');
    l1List.setText(l1Decisions);

    // L2 项目知识
    const l2Section = container.createDiv('context-section');
    l2Section.createEl('h3', { text: 'L2 - 项目知识' });
    const l2Files = await this.evif.listContext('L2');
    l2Files.forEach(file => {
      const item = l2Section.createEl('button');
      item.setText(file);
      item.onclick = () => this.loadL2File(file);
    });

    // 写入当前笔记到 L0
    const currentNote = this.app.workspace.getActiveFile();
    if (currentNote) {
      const content = await this.vault.read(currentNote);
      await this.evif.writeContext('L0', 'current', content);
    }
  }
}
```

---

## 五、实施计划

### Phase 1: 基础连接（第 1-2 周）

| 任务 | 工作量 | 输出 |
|------|--------|------|
| 项目初始化 | 1 天 | 插件骨架 |
| EVIF API 客户端 | 2 天 | evif-client.ts |
| 设置面板 | 2 天 | SettingsTab.ts |
| 基本命令 | 2 天 | 同步命令 |
| 测试运行 | 1 天 | 可用插件 |

### Phase 2: 上下文集成（第 3-4 周）

| 任务 | 工作量 | 输出 |
|------|--------|------|
| ContextPanel 视图 | 3 天 | 上下文面板 |
| L0/L1/L2 读写 | 2 天 | 上下文命令 |
| 自动同步引擎 | 3 天 | SyncEngine |
| 笔记联动 | 2 天 | 双向同步 |

### Phase 3: 技能系统（第 5-6 周）

| 任务 | 工作量 | 输出 |
|------|--------|------|
| SkillPanel 视图 | 3 天 | 技能面板 |
| 技能市场浏览 | 2 天 | 技能列表 |
| 技能执行 | 3 天 | 执行命令 |
| 结果展示 | 2 天 | 输出视图 |

### Phase 4: 记忆系统（第 7-8 周）

| 任务 | 工作量 | 输出 |
|------|--------|------|
| MemoryPanel 视图 | 3 天 | 记忆面板 |
| 向量搜索集成 | 3 天 | 搜索命令 |
| 自动记忆 | 2 天 | 记忆规则 |
| 记忆面板 | 2 天 | 相关记忆 |

### Phase 5: 高级功能（第 9-10 周）

| 任务 | 工作量 | 输出 |
|------|--------|------|
| MCP Bridge | 4 天 | Claude 集成 |
| QueuePanel | 2 天 | 任务队列 |
| 协作功能 | 4 天 | PipeFS 集成 |

---

## 六、核心插件命令

### 6.1 Obsidian 命令面板命令

| 命令 | 快捷键 | 功能 |
|------|--------|------|
| `EVIF: 同步笔记` | `Ctrl+Shift+S` | 同步当前笔记 |
| `EVIF: 写入 L0` | `Ctrl+Shift+L0` | 写入当前任务 |
| `EVIF: 写入 L1` | `Ctrl+Shift+L1` | 追加决策 |
| `EVIF: 搜索记忆` | `Ctrl+Shift+M` | 语义搜索 |
| `EVIF: 执行技能` | `Ctrl+Shift+K` | 运行技能 |
| `EVIF: 查看上下文` | `Ctrl+Shift+C` | 打开面板 |
| `EVIF: 记忆当前笔记` | `Ctrl+Shift+R` | 存储记忆 |
| `EVIF: 打开任务队列` | `Ctrl+Shift+Q` | 查看队列 |

### 6.2 侧边栏面板

| 面板 | 图标 | 功能 |
|------|------|------|
| Context Panel | 🧠 | L0/L1/L2 上下文 |
| Memory Panel | 💭 | 向量记忆搜索 |
| Skill Panel | ⚡ | 技能市场 |
| Queue Panel | 📋 | 任务队列 |

---

## 七、差异化价值

### 7.1 EVIF Obsidian 插件 vs 其他方案

| 功能 | EVIF | Notion | Roam | Logseq |
|------|------|--------|------|--------|
| 本地优先 | ✅ | ❌ | ✅ | ✅ |
| 云同步 | ✅ | ✅ | ❌ | ❌ |
| AI Agent 集成 | ✅ | ❌ | ❌ | ❌ |
| 向量记忆 | ✅ | ❌ | ❌ | ❌ |
| 技能系统 | ✅ | ❌ | ❌ | ❌ |
| MCP 协议 | ✅ | ❌ | ❌ | ❌ |
| 多租户 | ✅ | ✅ | ❌ | ❌ |

### 7.2 核心卖点

1. **Obsidian + AI Agent**：Obsidian 用户可以直接使用 AI Agent 的上下文管理能力
2. **本地优先**：数据存储在本地 EVIF 后端，隐私安全
3. **技能复用**：使用 SKILL.md 标准化工作流
4. **记忆增强**：向量搜索让笔记互联

---

## 八、技术要求

### 8.1 Obsidian 版本

- **最低版本**：1.0.0
- **推荐版本**：1.12.7+
- **API 类型**：TypeScript API

### 8.2 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| Obsidian | 1.0.0+ | 核心框架 |
| @codemirror | 最新 | 编辑器集成 |
| react | 18.x | UI 面板 |

### 8.3 EVIF 后端要求

| 功能 | API 端点 | 必须 |
|------|----------|------|
| 上下文读取 | `/context/{layer}/{path}` | ✅ |
| 上下文写入 | `/context/{layer}/{path}` | ✅ |
| 技能列表 | `/skills` | ✅ |
| 技能执行 | `/skills/{name}/execute` | ✅ |
| 记忆搜索 | `/memories/search` | ✅ |
| 记忆存储 | `/memories` | ✅ |
| MCP 协议 | MCP Server | 可选 |

---

## 九、下一步行动

### 本周

1. [ ] 创建 `evif-obsidian-plugin` 仓库
2. [ ] 实现基础 API 客户端
3. [ ] 创建设置面板

### 本月

1. [ ] 完成 Phase 1-2（基础连接 + 上下文）
2. [ ] 发布 Beta 版本
3. [ ] 收集反馈

---

## 参考资料

- [Obsidian Plugin Developer Documentation](https://docs.obsidian.md/Plugins/Getting+started/Build+a+plugin)
- [Obsidian Plugin API Reference](https://docs.obsidian.md/Plugins/API+Reference)
- [Obsidian Community Plugins](https://obsidian.md/plugins)
