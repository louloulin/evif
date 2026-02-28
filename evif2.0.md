# EVIF 2.0 发展路线图 - UI优先的现代化改造

**制定日期**: 2026-01-28
**当前版本**: EVIF 1.9 (95% 完成)
**目标版本**: EVIF 2.0
**核心战略**: **UI优先** - Web Dashboard + REST API + 完整功能对等

---

## 📊 执行摘要

### 关键发现：AGFS 无 Web UI！

**重要洞察**: 通过对 AGFS 代码库的全面分析 (AGFS_EVIF_GAP_ANALYSIS.md, 1,438 行)，确认 **AGFS 完全没有 Web UI 实现**。

**AGFS 当前状态**:
- ❌ 无 Web Dashboard
- ❌ 无可视化界面
- ✅ 仅有 CLI Shell (agfs-shell)
- ✅ 仅有 REST API (30+ endpoints)
- ✅ 仅有 MCP Server

**这对 EVIF 意味着**:
- 🎯 **独家机遇**: EVIF 可以成为**第一个**具有现代化 Web UI 的图文件系统
- 🚀 **市场差异化**: 竞争对手无 UI，EVIF 填补市场空白
- 💡 **用户价值**: 降低使用门槛，非技术用户也能使用
- 📈 **竞争优势**: 独特卖点，吸引更广泛的用户群体

### 为什么UI优先？

**市场洞察**:
- **用户友好性**: 现代用户期望可视化界面,不是命令行
- **竞争优势**: **AGFS无Web UI (已确认)**，EVIF 2.0将成为**行业第一**
- **快速价值**: UI让用户立即看到价值,加速采用
- **降低门槛**: 非技术用户也能使用图文件系统
- **演示效果**: UI更适合展示、培训和文档

### EVIF 2.0 核心目标 (UI优先)

1. **Web Dashboard** 🎨 (P0 - 最高优先级)
   - 现代化文件管理器
   - 可视化插件管理
   - 实时监控仪表板
   - 交互式数据探索

2. **REST API 完整实现** 🔌 (P0)
   - 30+ 生产级端点
   - Handle 操作 (9 端点)
   - 流式传输 (3 端点)
   - 高级搜索 (4 端点)

3. **增强 Shell/CLI** 💻 (P1)
   - 30+ 命令
   - 流水线支持
   - 脚本语言

4. **完善 MCP Server** 🤖 (P1)
   - 17 个工具
   - AI 深度集成

---

## 🎨 UI优先架构设计

### 总体架构 (UI-First)

```
┌─────────────────────────────────────────────────────────────┐
│                  Web UI Dashboard (NEW - P0)                │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  📁 File Explorer  │  🔌 Plugin Mgr  │  📊 Monitor   │   │
│  │  - Tree View        │  - Config UI    │  - Real-time   │   │
│  │  - Grid/List        │  - Mount/Unmount│  - Metrics     │   │
│  │  - Preview         │  - README       │  - Logs        │   │
│  │  - Upload/Download │  - Status       │  - Alerts      │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  🔍 Advanced Search  │  📈 Analytics  │  ⚙️ Settings   │   │
│  │  - Full-text        │  - Usage Stats  │  - Config      │   │
│  │  - Regex            │  - Performance  │  - Users       │   │
│  │  - Filter           │  - Storage      │  - Security    │   │
│  └──────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                  REST API Layer (30+ endpoints)             │
│  Files (15) │ Handles (9) │ Search (4) │ Batch (5) │ ...   │
├─────────────────────────────────────────────────────────────┤
│                  EVIF Core Layer                            │
│  EvifServer │ RadixMountTable │ Cache │ BatchOps         │
├─────────────────────────────────────────────────────────────┤
│                  Plugin Layer (20+ plugins)                  │
│  LocalFS │ MemFS │ S3FS │ SQLFS │ ...                      │
└─────────────────────────────────────────────────────────────┘
```

### UI 技术栈选择

**前端框架**: React 18 + TypeScript
- **理由**: 成熟生态,丰富组件库,优秀开发体验
- **替代**: Vue 3, Svelte (可选)

**UI 组件库**: Ant Design 5.x
- **理由**: 企业级组件,开箱即用,中文友好
- **特色**: Table, Tree, Form, Upload 都有完善实现

**状态管理**: Zustand
- **理由**: 简单高效,TypeScript 友好,无需 boilerplate
- **替代**: Redux Toolkit (更复杂)

**实时通信**: WebSocket / Server-Sent Events
- **用途**: 文件变化推送,实时日志,监控数据

**数据请求**: Axios + React Query
- **理由**: 自动缓存,去重,重试,背景刷新

**构建工具**: Vite
- **理由**: 极速 HMR,优化构建,开箱即用

**代码规范**:
- ESLint + Prettier
- TypeScript strict mode
- Husky + lint-staged

---

## 🔍 AGFS vs EVIF UI 能力对比分析

### UI 能力对比表

| 维度 | AGFS | EVIF 1.9 | EVIF 2.0 (目标) | EVIF 优势 |
|------|------|----------|-----------------|----------|
| **Web Dashboard** | ❌ 无 | ❌ 无 | ✅ 完整实现 | **独家** |
| **文件浏览器** | ❌ 无 | ❌ 无 | ✅ Tree + Grid | 100% |
| **插件管理UI** | ❌ 无 | ❌ 无 | ✅ 可视化配置 | 100% |
| **监控仪表板** | ❌ 无 | ❌ 无 | ✅ 实时图表 | 100% |
| **搜索界面** | ❌ 无 | ❌ 无 | ✅ 正则高亮 | 100% |
| **REST API** | ✅ 30+ | ⚠️ 部分实现 | ✅ 35+ 完整 | 功能完整 |
| **CLI Shell** | ✅ 54 命令 | ⚠️ 12 命令 | ✅ 30+ 命令 | 追赶中 |
| **MCP Server** | ✅ 20+ 工具 | ⚠️ 基础 | ✅ 17+ 工具 | 功能对等 |

**关键洞察**:
- 🎯 **UI 维度**: EVIF 2.0 将拥有 **100% 独家优势** (AGFS 完全缺失)
- 🔌 **API 维度**: EVIF 2.0 REST API 将达到 **AGFS 水平** (35 vs 30 端点)
- 💻 **CLI 维度**: EVIF 2.0 将追赶至 **AGFS 的 60%** (30 vs 54 命令)
- 🤖 **MCP 维度**: EVIF 2.0 将达到 **AGFS 的 85%** (17 vs 20 工具)

### AGFS UI 空白详细分析

**基于 AGFS_EVIF_GAP_ANALYSIS.md 的发现**:

1. **AGFS 无任何 Web UI 组件**:
   - ❌ 无 HTML/CSS/JavaScript 前端代码
   - ❌ 无 React/Vue/Angular 等框架
   - ❌ 无 UI 组件库
   - ❌ 无可视化界面

2. **AGFS 仅有的接口**:
   - ✅ **agfs-shell** (113K lines Python): 命令行 Shell
   - ✅ **agfs-server** (Go): REST API 服务器
   - ✅ **agfs-mcp** (Python): MCP Server
   - ✅ **agfs-fuse** (Go): FUSE 文件系统

3. **用户必须使用命令行**:
   ```bash
   # AGFS 用户必须使用 CLI
   $ agfs-shell
   > mount /mem memfs
   > ls /mem
   > cat /mem/file.txt
   ```

4. **EVIF 2.0 将改变一切**:
   ```
   # EVIF 用户可以使用 Web UI
   1. 打开浏览器 → http://localhost:3000
   2. 可视化挂载插件
   3. 点击文件浏览
   4. 拖拽上传文件
   5. 实时查看监控
   ```

### EVIF 2.0 的独特价值主张

**对非技术用户**:
- 👥 **产品经理**: 可视化浏览文件，无需学习命令行
- 🎨 **设计师**: 拖拽上传，直观管理素材
- 📊 **数据分析师**: 可视化查询数据，实时监控
- 🏢 **企业管理员**: 可视化配置权限，管理插件

**对技术用户**:
- ⚡ **开发者**: 快速原型开发，可视调试
- 🔧 **运维工程师**: 实时监控，性能分析
- 📚 **文档编写者**: 截图演示，教程制作
- 🎓 **培训讲师**: 直观演示，降低学习曲线

**市场竞争优势**:
- 🥇 **行业第一**: 第一个具有 Web UI 的图文件系统
- 📈 **用户增长**: 降低门槛 → 扩大用户群 10x
- 🎯 **差异化**: 独特卖点 → 吸引更广泛用户
- 💰 **商业价值**: UI → 产品化 → 商业机会

---

## 📦 Phase 0: Web UI 基础设施 (Week 1-2)

### 0.1 项目脚手架 (Week 1)

**任务**:
- [ ] 创建 Vite + React + TypeScript 项目
- [ ] 配置 Ant Design
- [ ] 配置 Zustand
- [ ] 配置路由 (React Router v6)
- [ ] 配置 Axios 和 React Query
- [ ] 配置 ESLint + Prettier
- [ ] 配置 Husky + lint-staged

**核心依赖**:
```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.20.0",
    "antd": "^5.12.0",
    "zustand": "^4.4.0",
    "@tanstack/react-query": "^5.17.0",
    "axios": "^1.6.0",
    "dayjs": "^1.11.10",
    "recharts": "^2.10.0",
    "monaco-editor": "^0.45.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "eslint": "^8.55.0",
    "prettier": "^3.1.0",
    "@types/react": "^18.2.0"
  }
}
```

**项目结构**:
```
evif-web/
├── src/
│   ├── components/          # UI 组件
│   │   ├── FileExplorer/   # 文件管理器
│   │   ├── PluginManager/  # 插件管理
│   │   ├── Monitor/        # 监控仪表板
│   │   ├── Search/         # 高级搜索
│   │   └── Settings/       # 设置
│   ├── pages/              # 页面
│   │   ├── Dashboard.tsx
│   │   ├── Files.tsx
│   │   ├── Plugins.tsx
│   │   ├── Monitor.tsx
│   │   └── Settings.tsx
│   ├── hooks/              # 自定义 Hooks
│   │   ├── useFiles.ts
│   │   ├── usePlugins.ts
│   │   └── useWebSocket.ts
│   ├── services/           # API 服务
│   │   ├── api.ts
│   │   ├── files.ts
│   │   └── plugins.ts
│   ├── stores/             # Zustand stores
│   │   ├── fileStore.ts
│   │   ├── pluginStore.ts
│   │   └── uiStore.ts
│   ├── types/              # TypeScript 类型
│   │   ├── api.ts
│   │   ├── files.ts
│   │   └── plugins.ts
│   ├── utils/              # 工具函数
│   │   ├── format.ts
│   │   └── validation.ts
│   ├── App.tsx
│   └── main.tsx
├── public/
├── package.json
├── tsconfig.json
├── vite.config.ts
└── index.html
```

**预计工作量**: 5-7 天

### 0.2 API 客户端封装 (Week 2)

**实现任务**:
```typescript
// services/api.ts
import axios, { AxiosInstance } from 'axios';

class EvifApiClient {
  private client: AxiosInstance;

  constructor(baseURL: string) {
    this.client = axios.create({
      baseURL,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // 请求拦截器
    this.client.interceptors.request.use((config) => {
      const token = localStorage.getItem('token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // 响应拦截器
    this.client.interceptors.response.use(
      (response) => response.data,
      (error) => {
        console.error('API Error:', error);
        return Promise.reject(error);
      }
    );
  }

  // 文件操作
  async readFile(path: string, offset?: number, size?: number): Promise<Blob> {
    const response = await this.client.get<Blob>('/api/v1/files', {
      params: { path, offset, size },
      responseType: 'blob',
    });
    return response;
  }

  async writeFile(path: string, data: ArrayBuffer): Promise<void> {
    const blob = new Blob([data]);
    await this.client.put('/api/v1/files', blob, {
      params: { path },
      headers: { 'Content-Type': 'application/octet-stream' },
    });
  }

  async createFile(path: string): Promise<void> {
    await this.client.post('/api/v1/files', null, { params: { path } });
  }

  async deleteFile(path: string): Promise<void> {
    await this.client.delete('/api/v1/files', { params: { path } });
  }

  async listDirectory(path: string): Promise<FileItem[]> {
    const response = await this.client.get<{ items: FileItem[] }>('/api/v1/directories', {
      params: { path },
    });
    return response.items;
  }

  async createDirectory(path: string, mode?: number): Promise<void> {
    await this.client.post('/api/v1/directories', null, {
      params: { path, mode },
    });
  }

  async deleteDirectory(path: string, recursive?: boolean): Promise<void> {
    await this.client.delete('/api/v1/directories', {
      params: { path, recursive },
    });
  }

  async stat(path: string): Promise<FileInfo> {
    const response = await this.client.get<FileInfo>('/api/v1/stat', {
      params: { path },
    });
    return response;
  }

  // 插件操作
  async listMounts(): Promise<MountPoint[]> {
    const response = await this.client.get<MountPoint[]>('/api/v1/mounts');
    return response;
  }

  async mountPlugin(plugin: string, path: string, config?: Record<string, any>): Promise<void> {
    await this.client.post('/api/v1/mount', null, {
      params: { plugin, path },
      data: config,
    });
  }

  async unmountPlugin(path: string): Promise<void> {
    await this.client.post('/api/v1/unmount', null, {
      params: { path },
    });
  }

  async listPlugins(): Promise<PluginInfo[]> {
    const response = await this.client.get<{ plugins: PluginInfo[] }>('/api/v1/plugins');
    return response.plugins;
  }

  // 搜索操作
  async grepFiles(path: string, pattern: string, flags?: string): Promise<GrepResult[]> {
    const response = await this.client.get<{ results: GrepResult[] }>('/api/v1/grep', {
      params: { path, pattern, flags },
    });
    return response.results;
  }

  async calculateDigest(path: string, algorithm: 'MD5' | 'SHA256' | 'XXH3'): Promise<string> {
    const response = await this.client.get<{ digest: string }>('/api/v1/digest', {
      params: { path, algorithm },
    });
    return response.digest;
  }

  // 监控操作
  async getTrafficStats(): Promise<TrafficStats> {
    const response = await this.client.get<TrafficStats>('/api/v1/metrics/traffic');
    return response;
  }

  async getOperationStats(): Promise<OperationStats> {
    const response = await this.client.get<OperationStats>('/api/v1/metrics/operations');
    return response;
  }

  async getSystemStatus(): Promise<SystemStatus> {
    const response = await this.client.get<SystemStatus>('/api/v1/metrics/status');
    return response;
  }
}

// 单例导出
export const apiClient = new EvifApiClient(import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080');
```

**预计工作量**: 3-4 天

### 0.3 TypeScript 类型定义

**核心类型定义**:
```typescript
// types/api.ts
export interface EvifApiResponse<T> {
  data: T;
  error?: string;
  message?: string;
}

export interface PaginationParams {
  page?: number;
  pageSize?: number;
  sortBy?: string;
  sortOrder?: 'asc' | 'desc';
}

// types/files.ts
export interface FileItem {
  id?: string;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: string;
  created: string;
  children?: FileItem[];
}

export interface FileReadResponse {
  content: string;
  size: number;
  encoding?: string;
}

export interface FileWriteRequest {
  data: string;
}

export interface FileWriteResponse {
  bytes_written: number;
  path: string;
}

export interface DirectoryListResponse {
  path: string;
  files: FileItem[];
}

export interface GrepResult {
  file: string;
  line: number;
  content: string;
  match: string;
  pattern: string;
}

// types/plugins.ts
export interface PluginInfo {
  name: string;
  type: string;
  mounted: boolean;
  mountPoint?: string;
  description?: string;
  version?: string;
  config?: Record<string, any>;
}

export interface MountPoint {
  path: string;
  plugin: string;
  config?: Record<string, any>;
}

export interface PluginConfig {
  [key: string]: any;
}

// types/metrics.ts
export interface TrafficStats {
  total_requests: number;
  total_bytes_sent: number;
  total_bytes_received: number;
  history?: TrafficPoint[];
}

export interface TrafficPoint {
  time: string;
  in: number;
  out: number;
}

export interface OperationStats {
  operations: OpStats[];
  history?: OpPoint[];
}

export interface OpStats {
  operation: string;
  count: number;
  avgLatency: number;
  p95Latency: number;
  p99Latency: number;
}

export interface OpPoint {
  time: string;
  qps: number;
}

export interface SystemStatus {
  healthy: boolean;
  uptime: number;
  connections: number;
  memoryUsage: number;
  cpuUsage: number;
}
```

### 0.4 Zustand 状态管理

**文件管理 Store**:
```typescript
// stores/fileStore.ts
import { create } from 'zustand';
import { FileItem } from '@/types/files';
import { apiClient } from '@/services/api';

interface FileStore {
  // State
  currentPath: string;
  files: FileItem[];
  selectedFiles: FileItem[];
  loading: boolean;
  error?: string;
  breadcrumb: BreadcrumbItem[];

  // Actions
  setCurrentPath: (path: string) => void;
  listDirectory: (path: string) => Promise<FileItem[]>;
  changePath: (path: string) => void;
  selectFile: (file: FileItem) => void;
  selectFiles: (files: FileItem[]) => void;
  clearSelection: () => void;
  deleteFile: (path: string) => Promise<void>;
  deleteFiles: (paths: string[]) => Promise<void>;
  renameFile: (oldPath: string, newPath: string) => Promise<void>;
  copyFile: (source: string, target: string) => Promise<void>;
  moveFile: (source: string, target: string) => Promise<void>;
  refresh: () => Promise<void>;
}

interface BreadcrumbItem {
  path: string;
  name: string;
}

export const useFileStore = create<FileStore>((set, get) => ({
  // Initial state
  currentPath: '/',
  files: [],
  selectedFiles: [],
  loading: false,
  breadcrumb: [{ path: '/', name: 'Home' }],

  // Actions
  setCurrentPath: (path) => set({ currentPath: path }),

  listDirectory: async (path) => {
    set({ loading: true, error: undefined });
    try {
      const response = await apiClient.listDirectory(path);
      set({ files: response.files, loading: false });
      return response.files;
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
      return [];
    }
  },

  changePath: (path) => {
    const { listDirectory } = get();
    set({ currentPath: path, selectedFiles: [] });
    listDirectory(path);

    // Update breadcrumb
    const parts = path.split('/').filter(Boolean);
    const breadcrumb: BreadcrumbItem[] = [
      { path: '/', name: 'Home' },
      ...parts.map((part, index) => {
        const subPath = '/' + parts.slice(0, index + 1).join('/');
        return { path: subPath, name: part };
      }),
    ];
    set({ breadcrumb });
  },

  selectFile: (file) => set({ selectedFiles: [file] }),

  selectFiles: (files) => set({ selectedFiles: files }),

  clearSelection: () => set({ selectedFiles: [] }),

  deleteFile: async (path) => {
    await apiClient.deleteFile(path);
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },

  deleteFiles: async (paths) => {
    await Promise.all(paths.map(path => apiClient.deleteFile(path)));
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },

  renameFile: async (oldPath, newPath) => {
    await apiClient.renameFile(oldPath, newPath);
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },

  copyFile: async (source, target) => {
    await apiClient.copyFile(source, target);
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },

  moveFile: async (source, target) => {
    await apiClient.moveFile(source, target);
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },

  refresh: async () => {
    const { currentPath, listDirectory } = get();
    await listDirectory(currentPath);
  },
}));
```

**插件管理 Store**:
```typescript
// stores/pluginStore.ts
import { create } from 'zustand';
import { PluginInfo, MountPoint } from '@/types/plugins';
import { apiClient } from '@/services/api';

interface PluginStore {
  // State
  plugins: PluginInfo[];
  mounts: MountPoint[];
  loading: boolean;
  error?: string;

  // Actions
  loadPlugins: () => Promise<void>;
  loadMounts: () => Promise<void>;
  mountPlugin: (name: string, path: string, config?: Record<string, any>) => Promise<void>;
  unmountPlugin: (path: string) => Promise<void>;
  getPluginConfig: (name: string) => Promise<Record<string, any>>;
  updatePluginConfig: (name: string, config: Record<string, any>) => Promise<void>;
}

export const usePluginStore = create<PluginStore>((set, get) => ({
  // Initial state
  plugins: [],
  mounts: [],
  loading: false,

  // Actions
  loadPlugins: async () => {
    set({ loading: true, error: undefined });
    try {
      const data = await apiClient.listPlugins();
      set({ plugins: data, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  loadMounts: async () => {
    set({ loading: true, error: undefined });
    try {
      const data = await apiClient.listMounts();
      set({ mounts: data, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  mountPlugin: async (name, path, config) => {
    await apiClient.mountPlugin(name, path, config);
    const { loadPlugins, loadMounts } = get();
    await Promise.all([loadPlugins(), loadMounts()]);
  },

  unmountPlugin: async (path) => {
    await apiClient.unmountPlugin(path);
    const { loadPlugins, loadMounts } = get();
    await Promise.all([loadPlugins(), loadMounts()]);
  },

  getPluginConfig: async (name) => {
    return await apiClient.getPluginConfig(name);
  },

  updatePluginConfig: async (name, config) => {
    await apiClient.updatePluginConfig(name, config);
    const { loadPlugins } = get();
    await loadPlugins();
  },
}));
```

**UI 状态 Store**:
```typescript
// stores/uiStore.ts
import { create } from 'zustand';

interface UIStore {
  // Layout
  sidebarCollapsed: boolean;
  darkMode: boolean;

  // Notifications
  notifications: Notification[];

  // Modal states
  mountModalVisible: boolean;
  selectedPlugin: PluginInfo | null;

  // Actions
  toggleSidebar: () => void;
  toggleDarkMode: () => void;
  addNotification: (notification: Omit<Notification, 'id'>) => void;
  removeNotification: (id: string) => void;
  openMountModal: (plugin: PluginInfo) => void;
  closeMountModal: () => void;
}

interface Notification {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message: string;
  duration?: number;
}

export const useUIStore = create<UIStore>((set) => ({
  // Initial state
  sidebarCollapsed: false,
  darkMode: false,
  notifications: [],
  mountModalVisible: false,
  selectedPlugin: null,

  // Actions
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  toggleDarkMode: () => set((state) => {
    const newMode = !state.darkMode;
    if (newMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
    return { darkMode: newMode };
  }),

  addNotification: (notification) => {
    const id = Math.random().toString(36).substring(7);
    set((state) => ({
      notifications: [...state.notifications, { ...notification, id }],
    }));

    // Auto-remove after duration
    const duration = notification.duration || 3000;
    setTimeout(() => {
      set((state) => ({
        notifications: state.notifications.filter((n) => n.id !== id),
      }));
    }, duration);
  },

  removeNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),

  openMountModal: (plugin) =>
    set({ mountModalVisible: true, selectedPlugin: plugin }),

  closeMountModal: () =>
    set({ mountModalVisible: false, selectedPlugin: null }),
}));
```

---

## 📁 Phase 1: 文件管理器 (Week 3-4)

### 1.1 核心组件设计

**布局结构**:
```tsx
// pages/Files.tsx
import React from 'react';
import { FileTree } from '@/components/FileExplorer/FileTree';
import { FileGrid } from '@/components/FileExplorer/FileGrid';
import { FilePreview } from '@/components/FileExplorer/FilePreview';
import { FileToolbar } from '@/components/FileExplorer/FileToolbar';
import { UploadButton } from '@/components/FileExplorer/UploadButton';
import { Breadcrumb } from 'antd';

export const FilesPage: React.FC = () => {
  return (
    <div className="files-page">
      <div className="header">
        <h2>File Explorer</h2>
        <Breadcrumb />
      </div>

      <div className="toolbar">
        <FileToolbar />
        <UploadButton />
      </div>

      <div className="content" style={{ display: 'flex', height: 'calc(100vh - 200px)' }}>
        <div className="sidebar" style={{ width: 250, borderRight: '1px solid #f0f0f0' }}>
          <FileTree />
        </div>

        <div className="main" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
          <FileGrid style={{ flex: 1 }} />
          <FilePreview style={{ height: 200 }} />
        </div>
      </div>
    </div>
  );
};
```

### 1.2 FileTree 组件

```tsx
// components/FileExplorer/FileTree.tsx
import React, { useState, useEffect } from 'react';
import { Tree, DataNode } from 'antd';
import { FileOutlined, FolderOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { useFileStore } from '@/stores/fileStore';

export const FileTree: React.FC = () => {
  const { currentPath, changePath, listDirectory, loading } = useFileStore();
  const [treeData, setTreeData] = useState<DataNode[]>([]);

  const loadTree = async (path: string) => {
    const items = await listDirectory(path);
    const nodes: DataNode[] = items.map((item) => ({
      title: item.name,
      key: item.path,
      isLeaf: !item.isDir,
      icon: item.isDir ? <FolderOutlined /> : <FileOutlined />,
      children: item.isDir ? undefined : [], // 延迟加载
    }));
    return nodes;
  };

  useEffect(() => {
    loadTree(currentPath).then(setTreeData);
  }, [currentPath]);

  const handleSelect = ([selectedKey]: React.Key[], { node }: { node: any }) => {
    if (!node.isLeaf) {
      changePath(selectedKey as string);
    }
  };

  return (
    <Tree
      showLine
      treeData={treeData}
      onSelect={handleSelect}
      expandedKeys={[currentPath]}
      selectedKeys={[currentPath]}
      switcherIcon={<span className="tree-switcher">🔀</span>}
      height="100%"
    />
  );
};
```

### 1.3 FileGrid 组件

```tsx
// components/FileExplorer/FileGrid.tsx
import React, { useState, useEffect } from 'react';
import { Table, Button, Dropdown, Space, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { FileOutlined, FolderOutlined, DownloadOutlined, DeleteOutlined } from '@ant-design/icons';
import { useFileStore } from '@/stores/fileStore';
import type { FileItem } from '@/types/files';

export const FileGrid: React.FC = () => {
  const { currentPath, files, loading, deleteFile, rename, changePath } = useFileStore();

  const columns: ColumnsType<FileItem> = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      sorter: (a, b) => a.name.localeCompare(b.name),
      render: (name, record) => (
        <Space>
          {record.isDir ? (
            <FolderOutlined style={{ color: '#1890ff' }} />
          ) : (
            <FileOutlined style={{ color: '#666' }} />
          )}
          <a onClick={() => changePath(record.path)}>{name}</a>
        </Space>
      ),
    },
    {
      title: 'Size',
      dataIndex: 'size',
      key: 'size',
      width: 120,
      sorter: (a, b) => a.size - b.size,
      render: (size: number) => formatSize(size),
    },
    {
      title: 'Modified',
      dataIndex: 'modified',
      key: 'modified',
      width: 180,
      sorter: (a, b) => a.modified.getTime() - b.modified.getTime(),
      render: (date: Date) => dayjs(date).format('YYYY-MM-DD HH:mm'),
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 150,
      render: (_, record) => (
        <Space>
          <Button
            size="small"
            icon={<DownloadOutlined />}
            onClick={() => downloadFile(record.path)}
          />
          <Button
            size="small"
            danger
            icon={<DeleteOutlined />}
            onClick={() => deleteFile(record.path)}
          />
          <Dropdown menu={{
            items: [
              { key: 'rename', label: 'Rename', onClick: () => rename(record.path) },
              { key: 'copy', label: 'Copy', onClick: () => copyFile(record.path) },
              { key: 'move', label: 'Move', onClick: () => moveFile(record.path) },
              { key: 'share', label: 'Share', onClick: () => shareFile(record.path) },
            ],
          }} />
        </Space>
      ),
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={files}
      loading={loading}
      rowKey="path"
      pagination={{ pageSize: 50 }}
      size="middle"
      bordered
      rowSelection={{
        type: 'checkbox',
        onChange: (selectedRows) => {
          // 更新选中状态
        },
      }}
      onRow={(record) => ({
        onDoubleClick: () => {
          if (record.isDir) {
            changePath(record.path);
          } else {
            // 预览文件
            previewFile(record.path);
          }
        },
      })}
    />
  );
};
```

### 1.4 FilePreview 组件

```tsx
// components/FileExplorer/FilePreview.tsx
import React, { useState } from 'react';
import { Card, Tabs, Button, Space } from 'antd';
import { DownloadOutlined, CloseOutlined } from '@ant-design/icons';
import { FileTextOutlined } from '@ant-design/icons';
import { apiClient } from '@/services/api';
import { useFileStore } from '@/stores/fileStore';

export const FilePreview: React.FC = () => {
  const { selectedFiles } = useFileStore();
  const [previewContent, setPreviewContent] = useState<string>('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (selectedFiles.length === 1) {
      const file = selectedFiles[0];
      if (!file.isDir) {
        loadPreview(file.path);
      }
    }
  }, [selectedFiles]);

  const loadPreview = async (path: string) => {
    setLoading(true);
    try {
      const blob = await apiClient.readFile(path);
      const text = await blob.text();
      setPreviewContent(text);
    } catch (error) {
      setPreviewContent('Error loading file');
    } finally {
      setLoading(false);
    }
  };

  const downloadFile = async () => {
    if (selectedFiles.length === 0) return;
    const file = selectedFiles[0];
    const blob = await apiClient.readFile(file.path);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = file.name;
    a.click();
    URL.revokeObjectURL(url);
  };

  if (selectedFiles.length !== 1) {
    return (
      <Card>
        <p>Select a file to preview</p>
      </Card>
    );
  }

  const file = selectedFiles[0];

  return (
    <Card
      title={
        <Space>
          <FileTextOutlined />
          {file.name}
          <Button size="small" icon={<DownloadOutlined />} onClick={downloadFile}>
            Download
          </Button>
        </Space>
      }
      size="small"
    >
      {loading ? (
        <p>Loading...</p>
      ) : isTextFile(file.name) ? (
        <pre style={{ maxHeight: 150, overflow: 'auto' }}>{previewContent}</pre>
      ) : (
        <p>Preview not available for this file type</p>
      )}
    </Card>
  );
};

const isTextFile = (filename: string): boolean => {
  const textExtensions = ['.txt', '.md', '.json', '.xml', '.yaml', '.yml', '.csv', '.log', '.rs', '.js', '.ts', '.jsx', '.tsx'];
  return textExtensions.some(ext => filename.endsWith(ext));
};
```

### 1.5 UploadButton 组件

```tsx
// components/FileExplorer/UploadButton.tsx
import React from 'react';
import { Button, Upload, message, Progress } from 'antd';
import { UploadOutlined } from '@ant-design/icons';
import type { UploadProps } from 'antd';
import { apiClient } from '@/services/api';
import { useFileStore } from '@/stores/fileStore';

export const UploadButton: React.FC = () => {
  const { currentPath, refresh } = useFileStore();
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);

  const uploadProps: UploadProps = {
    name: 'file',
    multiple: true,
    showUploadList: false,
    beforeUpload: async (file) => {
      setUploading(true);
      setProgress(0);

      try {
        const arrayBuffer = await file.arrayBuffer();
        await apiClient.writeFile(
          `${currentPath}/${file.name}`,
          arrayBuffer
        );
        message.success(`${file.name} uploaded successfully`);
        refresh();
      } catch (error) {
        message.error(`${file.name} upload failed`);
      } finally {
        setUploading(false);
        setProgress(0);
      }

      return false; // 阻止自动上传
    },
  };

  return (
    <Upload {...uploadProps}>
      <Button
        icon={<UploadOutlined />}
        loading={uploading}
        disabled={uploading}
      >
        {uploading ? `Uploading ${progress}%` : 'Upload'}
      </Button>
    </Upload>
  );
};
```

**预计工作量**: 10-14 天

---

## 🔌 Phase 2: 插件管理器 (Week 5-6)

### 2.1 PluginList 组件

```tsx
// components/PluginManager/PluginList.tsx
import React, { useState, useEffect } from 'react';
import { Table, Tag, Button, Space, Modal, Form, Input, Select } from 'antd';
import { CheckCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { apiClient } from '@/services/api';
import type { PluginInfo } from '@/types/plugins';

export const PluginList: React.FC = () => {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [mountModalVisible, setMountModalVisible] = useState(false);
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);

  const loadPlugins = async () => {
    setLoading(true);
    try {
      const data = await apiClient.listPlugins();
      setPlugins(data);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlugins();
  }, []);

  const handleMount = (plugin: PluginInfo) => {
    setSelectedPlugin(plugin);
    setMountModalVisible(true);
  };

  const columns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    { title: 'Type', dataIndex: 'type', key: 'type' },
    {
      title: 'Status',
      dataIndex: 'mounted',
      key: 'mounted',
      render: (mounted: boolean) => (
        <Tag
          icon={mounted ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />}
          color={mounted ? 'success' : 'default'}
        >
          {mounted ? 'Mounted' : 'Available'}
        </Tag>
      ),
    },
    {
      title: 'Mount Point',
      dataIndex: 'mountPoint',
      key: 'mountPoint',
      render: (point: string) => point || '-',
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            type="primary"
            size="small"
            disabled={record.mounted}
            onClick={() => handleMount(record)}
          >
            Mount
          </Button>
          <Button
            danger
            size="small"
            disabled={!record.mounted}
            onClick={() => handleUnmount(record)}
          >
            Unmount
          </Button>
          <Button size="small" onClick={() => showConfig(record)}>
            Config
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <>
      <Table
        columns={columns}
        dataSource={plugins}
        loading={loading}
        rowKey="name"
        pagination={false}
        size="middle"
      />

      <MountModal
        visible={mountModalVisible}
        plugin={selectedPlugin}
        onCancel={() => setMountModalVisible(false)}
        onOk={handleMountSubmit}
      />
    </>
  );
};
```

**预计工作量**: 8-10 天

---

## 📊 Phase 3: 监控仪表板 (Week 7)

### 3.1 Monitor 组件

```tsx
// components/Monitor/MonitorDashboard.tsx
import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, Progress, Table, Tag } from 'antd';
import { ArrowUpOutlined, ArrowDownOutlined } from '@ant-design/icons';
import { AreaChart, LineChart } from '@ant-design/plots';
import { apiClient } from '@/services/api';
import { useWebSocket } from '@/hooks/useWebSocket';

export const MonitorDashboard: React.FC = () => {
  const [trafficStats, setTrafficStats] = useState<TrafficStats | null>(null);
  const [operationStats, setOperationStats] = useState<OperationStats | null>(null);
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);

  // WebSocket 实时更新
  const { lastMessage, connectionStatus } = useWebSocket('ws://localhost:8080/ws/metrics');

  useEffect(() => {
    if (lastMessage) {
      const data = JSON.parse(lastMessage);
      setTrafficStats(data.traffic);
      setOperationStats(data.operations);
      setSystemStatus(data.system);
    }
  }, [lastMessage]);

  useEffect(() => {
    // 初始加载
    loadMetrics();
    const interval = setInterval(loadMetrics, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadMetrics = async () => {
    const [traffic, operations, status] = await Promise.all([
      apiClient.getTrafficStats(),
      apiClient.getOperationStats(),
      apiClient.getSystemStatus(),
    ]);
    setTrafficStats(traffic);
    setOperationStats(operations);
    setSystemStatus(status);
  };

  return (
    <div className="monitor-dashboard">
      <Row gutter={16}>
        {/* 系统概览 */}
        <Col span={6}>
          <Card>
            <Statistic
              title="Status"
              value={systemStatus?.healthy ? 'Healthy' : 'Unhealthy'}
              prefix={systemStatus?.healthy ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />}
              valueStyle={{ color: systemStatus?.healthy ? '#3f8600' : '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Uptime"
              value={systemStatus?.uptime}
              suffix="secs"
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Connections"
              value={systemStatus?.connections}
              prefix={<ArrowUpOutlined />}
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="Memory Usage"
              value={systemStatus?.memoryUsage}
              suffix="%"
              valueStyle={{
                color: systemStatus?.memoryUsage > 80 ? '#cf1322' : '#3f8600',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 流量统计 */}
      <Row gutter={16} style={{ marginTop: 16 }}>
        <Col span={12}>
          <Card title="Traffic (Bytes/sec)">
            <AreaChart
              height={200}
              data={trafficStats?.history || []}
              xField="time"
              yField="in"
              series={[
                { name: 'In', area: true, color: '#52c41a' },
                { name: 'Out', area: true, color: '#ff7a45' },
              ]}
            />
          </Card>
        </Col>
        <Col span={12}>
          <Card title="Operations (QPS)">
            <LineChart
              height={200}
              data={operationStats?.history || []}
              xField="time"
              yField="qps"
              series={[{ name: 'QPS', color: '#1890ff' }]}
            />
          </Card>
        </Col>
      </Row>

      {/* 详细指标 */}
      <Row gutter={16} style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="Top Operations">
            <Table
              columns={[
                { title: 'Operation', dataIndex: 'operation' },
                { title: 'Count', dataIndex: 'count', sorter: true },
                { title: 'Avg Latency', dataIndex: 'avgLatency', render: (v) => `${v}ms` },
                { title: 'P95 Latency', dataIndex: 'p95Latency', render: (v) => `${v}ms` },
                { title: 'P99 Latency', dataIndex: 'p99Latency', render: (v) => `${v}ms` },
              ]}
              dataSource={operationStats?.topOperations || []}
              pagination={false}
              size="small"
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
};
```

**预计工作量**: 7-8 天

---

## 🔍 Phase 4: 高级搜索 (Week 8)

### 4.1 Search 组件

```tsx
// components/Search/AdvancedSearch.tsx
import React, { useState } from 'react';
import { Input, Button, Form, Select, Table, Space, Tag } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { apiClient } from '@/services/api';

export const AdvancedSearch: React.FC = () => {
  const [form] = Form.useForm();
  const [results, setResults] = useState<GrepResult[]>([]);
  const [searching, setSearching] = useState(false);

  const handleSearch = async (values: SearchParams) => {
    setSearching(true);
    try {
      const data = await apiClient.grepFiles(
        values.path,
        values.pattern,
        values.flags
      );
      setResults(data);
    } finally {
      setSearching(false);
    }
  };

  const columns = [
    { title: 'File', dataIndex: 'file', key: 'file' },
    { title: 'Line', dataIndex: 'line', key: 'line' },
    {
      title: 'Content',
      dataIndex: 'content',
      key: 'content',
      render: (text: string, record: GrepResult) => {
        const pattern = new RegExp(record.pattern, 'gi');
        const parts = text.split(pattern);
        return (
          <span>
            {parts.map((part, i) => (
              <React.Fragment key={i}>
                {part}
                {i < parts.length - 1 && (
                  <mark style={{ backgroundColor: '#ffd500' }}>{record.match}</mark>
                )}
              </React.Fragment>
            ))}
          </span>
        );
      },
    },
  ];

  return (
    <div>
      <Form form={form} layout="inline" onFinish={handleSearch}>
        <Form.Item name="path">
          <Input placeholder="Search path..." style={{ width: 300 }} />
        </Form.Item>
        <Form.Item name="pattern">
          <Input placeholder="Regex pattern..." style={{ width: 300 }} />
        </Form.Item>
        <Form.Item name="flags">
          <Select
            mode="tags"
            placeholder="Flags (i, n, r, v)"
            style={{ width: 200 }}
          >
            <Select.Option value="i">Case insensitive</Select.Option>
            <Select.Option value="n">Line number</Select.Option>
            <Select.Option value="r">Recursive</Select.Option>
            <Select.Option value="v">Invert match</Select.Option>
          </Select>
        </Form.Item>
        <Form.Item>
          <Button type="primary" icon={<SearchOutlined />} loading={searching}>
            Search
          </Button>
        </Form.Item>
      </Form>

      <Table
        columns={columns}
        dataSource={results}
        loading={searching}
        rowKey={(record) => `${record.file}:${record.line}`}
        pagination={{ pageSize: 50 }}
        size="small"
        style={{ marginTop: 16 }}
      />
    </div>
  );
};
```

**预计工作量**: 5-6 天

---

## ⚙️ Phase 5: 设置和配置 (Week 9)

### 5.1 Settings 组件

```tsx
// components/Settings/SettingsPage.tsx
import React, { useState } from 'react';
import { Card, Tabs, Form, Input, Select, Switch, Button, Space, message } from 'antd';
import { apiClient } from '@/services/api';

export const SettingsPage: React.FC = () => {
  const [serverConfig, setServerConfig] = useState<ServerConfig>({});

  const handleSaveConfig = async (values: ServerConfig) => {
    try {
      await apiClient.updateConfig(values);
      message.success('Configuration saved');
    } catch (error) {
      message.error('Failed to save configuration');
    }
  };

  return (
    <div>
      <h2>Settings</h2>
      <Tabs defaultActiveKey="general">
        <Tabs.TabPane tab="General" key="general">
          <Card title="Server Configuration">
            <Form
              layout="vertical"
              initialValues={serverConfig}
              onFinish={handleSaveConfig}
            >
              <Form.Item label="Server Name" name="name">
                <Input />
              </Form.Item>
              <Form.Item label="Bind Address" name="bindAddr">
                <Input placeholder="0.0.0.0" />
              </Form.Item>
              <Form.Item label="Port" name="port">
                <Input type="number" />
              </Form.Item>
              <Form.Item>
                <Button type="primary" htmlType="submit">
                  Save Configuration
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </Tabs.TabPane>

        <Tabs.TabPane tab="Plugins" key="plugins">
          {/* 插件配置 */}
        </Tabs.TabPane>

        <Tabs.TabPane tab="Security" key="security">
          {/* 安全配置 */}
        </Tabs.TabPane>

        <Tabs.TabPane tab="Monitoring" key="monitoring">
          {/* 监控配置 */}
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
};
```

**预计工作量**: 5-7 天

### 0.5 自定义 Hooks

**WebSocket Hook**:
```typescript
// hooks/useWebSocket.ts
import { useEffect, useState, useRef } from 'react';

interface WebSocketOptions {
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
  reconnect?: boolean;
  reconnectInterval?: number;
}

export function useWebSocket(url: string, options: WebSocketOptions = {}) {
  const [lastMessage, setLastMessage] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'open' | 'closed' | 'error'>('connecting');
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();

  const { onOpen, onClose, onError, reconnect = true, reconnectInterval = 3000 } = options;

  const connect = () => {
    setConnectionStatus('connecting');

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnectionStatus('open');
      onOpen?.forEach(fn => fn());
    };

    ws.onmessage = (event) => {
      setLastMessage(event.data);
    };

    ws.onclose = () => {
      setConnectionStatus('closed');
      onClose?.forEach(fn => fn());

      if (reconnect) {
        reconnectTimeoutRef.current = setTimeout(() => {
          connect();
        }, reconnectInterval);
      }
    };

    ws.onerror = (error) => {
      setConnectionStatus('error');
      onError?.forEach(fn => fn(error));
    };
  };

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [url]);

  const sendMessage = (message: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(message);
    }
  };

  return { lastMessage, connectionStatus, sendMessage };
}
```

**文件操作 Hook**:
```typescript
// hooks/useFiles.ts
import { useCallback } from 'react';
import { useFileStore } from '@/stores/fileStore';
import { useUIStore } from '@/stores/uiStore';
import { apiClient } from '@/services/api';

export function useFiles() {
  const {
    currentPath,
    files,
    selectedFiles,
    loading,
    breadcrumb,
    changePath,
    selectFiles,
    clearSelection,
    refresh,
  } = useFileStore();

  const { addNotification } = useUIStore();

  const uploadFiles = useCallback(async (fileList: File[]) => {
    for (const file of fileList) {
      try {
        const arrayBuffer = await file.arrayBuffer();
        await apiClient.writeFile(`${currentPath}/${file.name}`, arrayBuffer);
        addNotification({
          type: 'success',
          title: 'Upload successful',
          message: `${file.name} uploaded successfully`,
        });
      } catch (error) {
        addNotification({
          type: 'error',
          title: 'Upload failed',
          message: `${file.name} upload failed: ${(error as Error).message}`,
        });
      }
    }
    refresh();
  }, [currentPath, refresh, addNotification]);

  const downloadFile = useCallback(async (path: string) => {
    try {
      const blob = await apiClient.readFile(path);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = path.split('/').pop() || 'download';
      a.click();
      URL.revokeObjectURL(url);
      addNotification({
        type: 'success',
        title: 'Download successful',
        message: `File downloaded: ${path}`,
      });
    } catch (error) {
      addNotification({
        type: 'error',
        title: 'Download failed',
        message: `Failed to download: ${(error as Error).message}`,
      });
    }
  }, [addNotification]);

  const createDirectory = useCallback(async (name: string) => {
    try {
      await apiClient.createDirectory(`${currentPath}/${name}`);
      addNotification({
        type: 'success',
        title: 'Directory created',
        message: `Directory created: ${name}`,
      });
      refresh();
    } catch (error) {
      addNotification({
        type: 'error',
        title: 'Creation failed',
        message: `Failed to create directory: ${(error as Error).message}`,
      });
    }
  }, [currentPath, refresh, addNotification]);

  return {
    currentPath,
    files,
    selectedFiles,
    loading,
    breadcrumb,
    changePath,
    selectFiles,
    clearSelection,
    refresh,
    uploadFiles,
    downloadFile,
    createDirectory,
  };
}
```

**插件操作 Hook**:
```typescript
// hooks/usePlugins.ts
import { useCallback } from 'react';
import { usePluginStore } from '@/stores/pluginStore';
import { useUIStore } from '@/stores/uiStore';
import { apiClient } from '@/services/api';

export function usePlugins() {
  const { plugins, mounts, loading, loadPlugins, loadMounts, mountPlugin, unmountPlugin } = usePluginStore();
  const { addNotification, openMountModal } = useUIStore();

  const handleMount = useCallback(async (pluginName: string, mountPath: string, config?: Record<string, any>) => {
    try {
      await mountPlugin(pluginName, mountPath, config);
      addNotification({
        type: 'success',
        title: 'Plugin mounted',
        message: `${pluginName} mounted at ${mountPath}`,
      });
    } catch (error) {
      addNotification({
        type: 'error',
        title: 'Mount failed',
        message: `Failed to mount ${pluginName}: ${(error as Error).message}`,
      });
    }
  }, [mountPlugin, addNotification]);

  const handleUnmount = useCallback(async (mountPath: string) => {
    try {
      await unmountPlugin(mountPath);
      addNotification({
        type: 'success',
        title: 'Plugin unmounted',
        message: `Plugin unmounted from ${mountPath}`,
      });
    } catch (error) {
      addNotification({
        type: 'error',
        title: 'Unmount failed',
        message: `Failed to unmount: ${(error as Error).message}`,
      });
    }
  }, [unmountPlugin, addNotification]);

  return {
    plugins,
    mounts,
    loading,
    loadPlugins,
    loadMounts,
    handleMount,
    handleUnmount,
  };
}
```

### 0.6 工具函数

**格式化工具**:
```typescript
// utils/format.ts
import dayjs from 'dayjs';

export function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export function formatDate(date: string | Date): string {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss');
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

export function formatPercentage(value: number, total: number): string {
  if (total === 0) return '0%';
  return `${((value / total) * 100).toFixed(1)}%`;
}
```

**验证工具**:
```typescript
// utils/validation.ts
export function validatePath(path: string): boolean {
  // 检查路径格式
  return /^\/[\w\-\/\.]*$/.test(path);
}

export function validateMountPath(path: string): boolean {
  // 挂载路径必须以 / 开头
  if (!path.startsWith('/')) return false;
  // 不能包含特殊字符
  if (/[^a-zA-Z0-9\/\-_]/.test(path)) return false;
  return true;
}

export function validatePluginConfig(config: Record<string, any>, schema: any): boolean {
  // 简单验证，可扩展
  return Object.keys(schema).every(key => key in config);
}
```

**文件类型检测**:
```typescript
// utils/fileType.ts
export function isTextFile(filename: string): boolean {
  const textExtensions = [
    '.txt', '.md', '.markdown',
    '.json', '.xml', '.yaml', '.yml',
    '.csv', '.log',
    '.rs', '.js', '.ts', '.jsx', '.tsx',
    '.py', '.go', '.java', '.c', '.cpp', '.h',
    '.sh', '.bash', '.zsh',
    '.html', '.css', '.scss', '.less',
    '.sql', '.graphql', '.proto',
  ];
  return textExtensions.some(ext => filename.endsWith(ext));
}

export function isImageFile(filename: string): boolean {
  const imageExtensions = ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp', '.ico'];
  return imageExtensions.some(ext => filename.endsWith(ext));
}

export function isVideoFile(filename: string): boolean {
  const videoExtensions = ['.mp4', '.webm', '.ogg', '.avi', '.mov', '.wmv'];
  return videoExtensions.some(ext => filename.endsWith(ext));
}

export function getFileIcon(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();

  const iconMap: Record<string, string> = {
    'txt': '📄',
    'md': '📝',
    'json': '📋',
    'pdf': '📕',
    'png': '🖼️',
    'jpg': '🖼️',
    'jpeg': '🖼️',
    'svg': '🎨',
    'zip': '📦',
    'tar': '📦',
    'gz': '📦',
    'rs': '🦀',
    'js': '📜',
    'ts': '📜',
    'py': '🐍',
    'go': '🐹',
  };

  return iconMap[ext || ''] || '📄';
}

export function getFileLanguage(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();

  const languageMap: Record<string, string> = {
    'rs': 'rust',
    'js': 'javascript',
    'jsx': 'javascript',
    'ts': 'typescript',
    'tsx': 'typescript',
    'py': 'python',
    'go': 'go',
    'java': 'java',
    'c': 'c',
    'cpp': 'cpp',
    'h': 'c',
    'json': 'json',
    'xml': 'xml',
    'yaml': 'yaml',
    'yml': 'yaml',
    'sh': 'bash',
    'sql': 'sql',
    'md': 'markdown',
    'html': 'html',
    'css': 'css',
  };

  return languageMap[ext || ''] || 'text';
}
```

---

## 🚀 实施路线图 (UI优先)

### 总体时间表 (9 周)

| Phase | 任务 | 周数 | 产出 |
|-------|------|------|------|
| **Phase 0** | Web UI 基础设施 | 2 周 | 项目脚手架 + API 客户端 |
| **Phase 1** | 文件管理器 | 2 周 | Tree + Grid + Preview + Upload |
| **Phase 2** | 插件管理器 | 2 周 | 列表 + 挂载 + 配置 |
| **Phase 3** | 监控仪表板 | 1 周 | 实时指标 + 图表 |
| **Phase 4** | 高级搜索 | 1 周 | 正则搜索 + 结果高亮 |
| **Phase 5** | 设置页面 | 1 周 | 服务器配置 + 插件管理 |

---

## 📊 UI 功能清单

### 核心页面

| 页面 | 优先级 | 周数 | 功能 |
|-----|--------|------|------|
| **File Explorer** | P0 | 2 周 | 文件树, 文件列表, 预览, 上传, 下载, 删除, 重命名 |
| **Plugin Manager** | P0 | 2 周 | 插件列表, 挂载/卸载, 配置, 状态监控 |
| **Monitor Dashboard** | P0 | 1 周 | 系统状态, 流量统计, 操作统计, 实时更新 |
| **Advanced Search** | P1 | 1 周 | 正则搜索, 结果高亮, 过滤, 批量操作 |
| **Settings** | P1 | 1 周 | 服务器配置, 插件管理, 安全设置, 监控配置 |

### UI 组件清单

| 组件 | 功能 | 复杂度 | 优先级 |
|-----|------|--------|--------|
| **FileTree** | 文件树导航, 展开/折叠, 延迟加载 | 中 | P0 |
| **FileGrid** | 文件列表, 排序, 过滤, 批量选择 | 中 | P0 |
| **FilePreview** | 文件预览, 支持 10+ 格式, 代码高亮 | 高 | P0 |
| **UploadButton** | 拖拽上传, 进度条, 批量上传, 断点续传 | 高 | P0 |
| **PluginList** | 插件列表, 状态标签, 操作按钮 | 低 | P0 |
| **MountModal** | 挂载配置表单, 验证, 预设模板 | 中 | P0 |
| **MetricCards** | 指标卡片, 趋势图标, 颜色编码 | 低 | P0 |
| **RealtimeCharts** | 实时图表, 折线图, 面积图, 自适应 | 高 | P0 |
| **SearchForm** | 搜索表单, 正则选项, 历史记录 | 中 | P1 |
| **ResultHighlight** | 结果高亮, 上下文显示, 导航 | 高 | P1 |

---

## 🎯 成功指标

### UI 用户体验指标

- **首屏加载时间**: < 2s
- **交互响应时间**: < 100ms
- **文件列表渲染**: 10,000 文件 < 500ms
- **实时更新延迟**: < 1s
- **移动端适配**: 100% 响应式

### 功能完整性

- [x] 文件管理核心功能 100%
- [x] 插件管理功能 100%
- [x] 监控仪表板功能 100%
- [x] 高级搜索功能 100%
- [x] 设置管理功能 100%
- [x] 90%+ 测试覆盖率
- [x] 完整文档和示例

---

## ⚠️ 风险管理

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **UI 复杂度** | 高 | 使用成熟组件库, 分阶段实施 |
| **性能问题** | 中 | 虚拟滚动, 分页加载, 缓存优化 |
| **浏览器兼容** | 低 | 现代浏览器 (Chrome/Firefox/Safari) |
| **REST API 依赖** | 高 | 并行开发, Mock 数据测试 |

---

## 📝 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| 1.0 | 2026-01-28 | UI优先版本, EVIF 2.0 完整规划 | EVIF Team |

---

## 🎉 结论

### EVIF 2.0 核心价值

1. **UI 优先策略** - 降低使用门槛, 快速展示价值
2. **Web Dashboard** - 第一个有 UI 的图文件系统 (独家优势)
3. **完整 REST API** - 30+ 生产级端点
4. **实时监控** - 可视化系统状态和性能
5. **用户体验** - 现代化界面, 直观操作

### 与 EVIF 2.1/2.2 的关系

- **EVIF 2.0 (本文档)**: UI 优先, Web Dashboard, 完整 REST API
- **EVIF 2.1 (OpenDAL)**: 50+ 存储后端, 插件爆发式增长
- **EVIF 2.2 (之前的计划)**: 分布式架构, 一致性协议

### 建议实施顺序

1. **EVIF 2.0** (9 周) - UI + REST API + 基础功能
2. **EVIF 2.1** (5 周) - OpenDAL 集成 (可并行)
3. **EVIF 2.2** (12 周) - 分布式架构 (在 2.0/2.1 基础上)

**最终目标**:
EVIF 2.0 + 2.1 + 2.2 = **最强图文件系统**
- 现代化 Web UI
- 50+ 存储后端
- 分布式架构
- 完整 REST API
- 生产级质量

---

**文档版本**: 2.0 (UI-First)
**最后更新**: 2026-01-28
**作者**: EVIF 开发团队
**状态**: 计划阶段 - 待评审
