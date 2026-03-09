import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';

// Components
import MenuBar from './components/MenuBar';
import ActivityBar from './components/ActivityBar';
import FileTree from './components/FileTree';
import Editor from './components/Editor';
import Panel from './components/Panel';
import StatusBar from './components/StatusBar';
import ContextMenu from './components/ContextMenu';
import PluginManagerView from './components/PluginManagerView';
import SearchUploadView from './components/SearchUploadView';
import { MonitorView } from './components/MonitorView';
import MemoryView from './components/memory/MemoryView';
import { KeyboardShortcutsDialog } from './components/KeyboardShortcutsDialog';
import { EditorTabs } from './components/editor/EditorTabs';
import { Breadcrumb } from './components/Breadcrumb';
import { QuickOpen } from './components/editor/QuickOpen';
import { CommandPalette } from './components/CommandPalette';
import { NotificationCenter } from './components/NotificationCenter';
import { Bell } from 'lucide-react';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useNotificationCenter } from './hooks/useNotificationCenter';
import { Toaster } from './components/ui/toaster';
import { toast } from '@/hooks/use-toast';
import { NetworkBanner } from './components/NetworkBanner';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from './components/ui/dialog';
import { Button } from './components/ui/button';
import type { ActivityView } from './components/ActivityBar';
import type { PanelTab } from './components/Panel';
import type { ProblemItem } from './components/Panel';
import type { EditorTab, QuickOpenItem } from './types/editor';

// Styles
import './App.css';
import { httpFetch } from '@/lib/http';

interface FileNode {
  path: string;
  name: string;
  is_dir: boolean;
  children?: FileNode[];
}

interface ContextMenuPosition {
  x: number;
  y: number;
  file: FileNode | null;
}

function App() {
  const [files, setFiles] = useState<FileNode[]>([]);
  const [childrenMap, setChildrenMap] = useState<Record<string, FileNode[]>>({});
  const [filesLoading, setFilesLoading] = useState(false);
  const [filesError, setFilesError] = useState<string | null>(null);

  // Tab state
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);

  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [panelVisible, setPanelVisible] = useState(true);
  const [activeView, setActiveView] = useState<ActivityView | null>('explorer');
  const [panelTab, setPanelTab] = useState<PanelTab>('terminal');
  const [operationProblems, setOperationProblems] = useState<ProblemItem[]>([]);
  const [loadingPaths, setLoadingPaths] = useState<Set<string>>(new Set());
  const loadingPathRef = useRef<string | null>(null);
  const [quickOpenOpen, setQuickOpenOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  // 通知中心
  const notificationCenter = useNotificationCenter();

  /**
   * Get language from file extension
   */
  const getLanguageFromFile = (filename: string): string => {
    const ext = filename.split('.').pop()?.toLowerCase();
    const langMap: Record<string, string> = {
      'ts': 'typescript',
      'tsx': 'typescript',
      'js': 'javascript',
      'jsx': 'javascript',
      'py': 'python',
      'rs': 'rust',
      'go': 'go',
      'java': 'java',
      'json': 'json',
      'md': 'markdown',
      'css': 'css',
      'html': 'html',
      'xml': 'xml',
      'yaml': 'yaml',
      'yml': 'yaml',
      'sh': 'shell',
      'txt': 'text',
    };
    return langMap[ext || ''] || 'text';
  };

  const problems: ProblemItem[] = useMemo(() => {
    const list: ProblemItem[] = [];
    if (filesError) {
      list.push({
        id: 'fs-list',
        message: filesError,
        source: 'API',
        severity: 'error'
      });
    }
    return [...list, ...operationProblems];
  }, [filesError, operationProblems]);

  // Get active tab
  const activeTab = useMemo(() => {
    return tabs.find(t => t.id === activeTabId) || null;
  }, [tabs, activeTabId]);

  // Check if current file is loading
  const isFileLoading = useMemo(() => {
    return activeTab ? loadingPathRef.current === activeTab.path : false;
  }, [activeTab]);

  // Convert files to QuickOpenItem format
  const quickOpenFiles = useMemo((): QuickOpenItem[] => {
    const convertNode = (node: FileNode): QuickOpenItem[] => {
      const items: QuickOpenItem[] = [{
        path: node.path,
        name: node.name,
        type: node.is_dir ? 'directory' : 'file',
        language: node.is_dir ? undefined : getLanguageFromFile(node.name)
      }];

      if (node.children) {
        for (const child of node.children) {
          items.push(...convertNode(child));
        }
      }

      return items;
    };

    const allItems: QuickOpenItem[] = [];
    for (const node of files) {
      allItems.push(...convertNode(node));
    }

    // Also include files from childrenMap
    for (const [path, children] of Object.entries(childrenMap)) {
      for (const child of children) {
        allItems.push(...convertNode(child));
      }
    }

    return allItems;
  }, [files, childrenMap]);

  // Fetch file list on mount
  useEffect(() => {
    fetchFiles();
  }, []);

  // Setup keyboard shortcuts
  useKeyboardShortcuts([
    // File shortcuts
    {
      key: 'p',
      ctrlKey: true,
      metaKey: true,
      description: '快速打开',
      action: () => setQuickOpenOpen(true),
    },
    {
      key: 's',
      ctrlKey: true,
      metaKey: true,
      description: '保存文件',
      action: () => {
        if (activeTabId) handleFileSave(activeTabId);
      },
    },
    {
      key: 'w',
      ctrlKey: true,
      metaKey: true,
      description: '关闭标签页',
      action: () => {
        if (activeTabId) handleTabClose(activeTabId);
      },
    },
    {
      key: 'n',
      ctrlKey: true,
      metaKey: true,
      shiftKey: true,
      description: '新建文件',
      action: () => handleNewFile(),
    },
    // Navigation shortcuts
    {
      key: 'e',
      ctrlKey: true,
      shiftKey: true,
      metaKey: true,
      description: '显示资源管理器',
      action: () => {
        setActiveView('explorer');
        setSidebarVisible(true);
      },
    },
    {
      key: '`',
      ctrlKey: true,
      metaKey: true,
      description: '显示终端',
      action: () => {
        setActiveView('terminal');
        setPanelVisible(true);
        setPanelTab('terminal');
      },
    },
    {
      key: 'b',
      ctrlKey: true,
      shiftKey: true,
      metaKey: true,
      description: '切换侧边栏',
      action: () => setSidebarVisible(v => !v),
    },
    {
      key: 'j',
      ctrlKey: true,
      shiftKey: true,
      metaKey: true,
      description: '显示问题',
      action: () => {
        setActiveView('problems');
        setPanelVisible(true);
        setPanelTab('problems');
      },
    },
    // Help shortcut
    {
      key: '/',
      ctrlKey: true,
      metaKey: true,
      shiftKey: true,
      description: '显示键盘快捷键',
      action: () => setShortcutsOpen(true),
    },
    // Command palette shortcut
    {
      key: 'p',
      ctrlKey: true,
      metaKey: true,
      shiftKey: true,
      description: '打开命令面板',
      action: () => setCommandPaletteOpen(true),
    },
  ]);

  /**
   * 拉取文件列表：path 为空或 '/' 时拉取根并更新 files，否则拉取该目录子项并写入 childrenMap
   */
  const fetchFiles = useCallback(async (path?: string) => {
    const listPath = path ?? '/';
    if (listPath === '/') {
      setFilesLoading(true);
      setFilesError(null);
    } else {
      setLoadingPaths((prev) => new Set(prev).add(listPath));
    }
    try {
      const response = await httpFetch('/api/v1/fs/list?path=' + encodeURIComponent(listPath));
      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        const msg = (data && (data.message || data.error)) || '加载失败';
        throw new Error(msg);
      }
      const nodes = data.nodes || [];
      if (listPath === '/') {
        setFiles(nodes);
        setFilesError(null);
      } else {
        setChildrenMap((prev) => ({ ...prev, [listPath]: nodes }));
      }
    } catch (error) {
      console.error('Error fetching files:', error);
      if (listPath === '/') {
        setFilesError(error instanceof Error ? error.message : '加载失败');
      }
    } finally {
      if (listPath === '/') setFilesLoading(false);
      if (listPath !== '/') {
        setLoadingPaths((prev) => {
          const next = new Set(prev);
          next.delete(listPath);
          return next;
        });
      }
    }
  }, []);

  /**
   * 选择并打开文件（创建或切换到标签页）
   */
  const handleFileSelect = useCallback(async (file: FileNode) => {
    if (file.is_dir) return;
    const path = file.path;

    // Check if tab already exists
    const existingTab = tabs.find(t => t.path === path);
    if (existingTab) {
      setActiveTabId(existingTab.id);
      return;
    }

    // Create new tab
    const tabId = `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const newTab: EditorTab = {
      id: tabId,
      path,
      name: file.name,
      content: '',
      language: getLanguageFromFile(file.name),
      modified: false,
    };

    setTabs(prev => [...prev, newTab]);
    setActiveTabId(tabId);

    // 添加到最近打开文件列表
    setRecentFiles(prev => {
      const filtered = prev.filter(p => p !== path);
      return [path, ...filtered].slice(0, 10);
    });

    // Load file content
    loadingPathRef.current = path;
    try {
      const response = await httpFetch(`/api/v1/fs/read?path=${encodeURIComponent(path)}`);
      const data = await response.json().catch(() => ({}));

      // Check if still the active tab
      setTabs(prev => {
        const currentTab = prev.find(t => t.id === tabId);
        if (!currentTab || currentTab.path !== path) return prev;

        if (loadingPathRef.current !== path) return prev;

        if (!response.ok) {
          const msg = (data && (data.message || data.error)) || '读取失败';
          setOperationProblems(prev => [...prev.slice(-2), {
            id: `read-${Date.now()}`,
            message: msg,
            source: '读取',
            severity: 'error'
          }]);
          return prev.map(t => t.id === tabId ? { ...t, content: `// Error: ${msg}` } : t);
        }

        return prev.map(t => t.id === tabId ? { ...t, content: data.content ?? '' } : t);
      });
    } catch (error) {
      console.error('Error reading file:', error);
      const msg = error instanceof Error ? error.message : '读取失败';

      setTabs(prev => prev.map(t =>
        t.id === tabId ? { ...t, content: `// Error: ${msg}` } : t
      ));

      setOperationProblems(prev => [...prev.slice(-2), {
        id: `read-${Date.now()}`,
        message: msg,
        source: '读取',
        severity: 'error'
      }]);
    } finally {
      if (loadingPathRef.current === path) loadingPathRef.current = null;
    }
  }, [tabs]);

  /**
   * 保存文件内容
   */
  const handleFileSave = useCallback(async (tabId?: string) => {
    const targetTabId = tabId || activeTabId;
    if (!targetTabId) return;

    const tab = tabs.find(t => t.id === targetTabId);
    if (!tab) return;

    try {
      const response = await httpFetch(`/api/v1/fs/write?path=${encodeURIComponent(tab.path)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: tab.content })
      });
      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        const msg = (data && (data.message || data.error)) || '保存失败';
        throw new Error(msg);
      }

      // Update tab as not modified
      setTabs(prev => prev.map(t =>
        t.id === targetTabId ? { ...t, modified: false } : t
      ));

      // 显示保存成功toast
      toast({
        title: "保存成功",
        description: `文件 ${tab.name} 已保存`,
      });

      setOperationProblems(prev => prev.filter(p => p.id !== 'save'));
    } catch (error) {
      console.error('Error saving file:', error);
      const msg = error instanceof Error ? error.message : '保存失败';
      setOperationProblems(prev => [...prev.slice(-2), {
        id: 'save',
        message: msg,
        source: '保存',
        severity: 'error'
      }]);
    }
  }, [tabs, activeTabId]);

  /**
   * 编辑器内容变更时同步到 tab state，并标记为已修改
   */
  const handleEditorChange = useCallback((value: string | undefined) => {
    if (!activeTabId) return;

    setTabs(prev => prev.map(t => {
      if (t.id === activeTabId) {
        const newContent = value ?? '';
        const wasModified = t.modified;
        const isModified = newContent !== t.content;
        return {
          ...t,
          content: newContent,
          modified: isModified
        };
      }
      return t;
    }));
  }, [activeTabId]);

  /**
   * 关闭标签页
   */
  const handleTabClose = useCallback((tabId: string) => {
    setTabs(prev => {
      const newTabs = prev.filter(t => t.id !== tabId);

      // If closing active tab, switch to another
      if (tabId === activeTabId) {
        const currentIndex = prev.findIndex(t => t.id === tabId);
        const nextTab = newTabs[currentIndex] || newTabs[newTabs.length - 1] || null;
        setActiveTabId(nextTab?.id || null);
      }

      return newTabs;
    });
  }, [activeTabId]);

  /**
   * 选择标签页
   */
  const handleTabSelect = useCallback((tabId: string) => {
    setActiveTabId(tabId);

    // 更新最近打开文件列表
    const tab = tabs.find(t => t.id === tabId);
    if (tab) {
      setRecentFiles(prev => {
        const filtered = prev.filter(p => p !== tab.path);
        return [tab.path, ...filtered].slice(0, 10);
      });
    }
  }, [tabs]);

  /**
   * 处理快速打开文件选择
   */
  const handleQuickOpenSelect = useCallback((item: QuickOpenItem) => {
    if (item.type === 'directory') return;

    const node: FileNode = {
      path: item.path,
      name: item.name,
      is_dir: false
    };

    // 添加到最近打开文件列表
    setRecentFiles(prev => {
      const filtered = prev.filter(path => path !== item.path);
      return [item.path, ...filtered].slice(0, 10);
    });

    handleFileSelect(node);
  }, [handleFileSelect]);

  const handleContextMenu = (e: React.MouseEvent, file: FileNode | null) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      file
    });
  };

  const handleContextMenuClose = () => {
    setContextMenu(null);
  };

  const handleRefresh = () => {
    setChildrenMap({});
    setOperationProblems([]);
    fetchFiles();
  };

  /**
   * 生成唯一文件名,自动处理冲突
   * @param basePath 基础路径 (如 "/local")
   * @param desiredName 期望的文件名 (如 "untitled")
   * @param existingFiles 现有文件列表
   * @returns 唯一的文件路径
   */
  const generateUniqueFilePath = (
    basePath: string,
    desiredName: string,
    existingFiles: FileNode[]
  ): string => {
    const desiredPath = `${basePath}/${desiredName}`;

    // 检查是否存在同名文件(检查路径是否匹配)
    const existingFile = existingFiles.find(f => {
      // 标准化路径进行比较(去掉开头的/)
      const normalizedPath = f.path.startsWith('/') ? f.path.substring(1) : f.path;
      const normalizedDesired = desiredPath.startsWith('/') ? desiredPath.substring(1) : desiredPath;
      return normalizedPath === normalizedDesired;
    });

    if (!existingFile) {
      // 无冲突,直接使用
      return desiredPath;
    }

    // 提取文件名和扩展名
    const lastDotIndex = desiredName.lastIndexOf('.');
    const nameWithoutExt = lastDotIndex > 0
      ? desiredName.substring(0, lastDotIndex)
      : desiredName;
    const extension = lastDotIndex > 0
      ? desiredName.substring(lastDotIndex)
      : '';

    // 构建正则匹配现有序号文件(仅匹配文件名)
    const numberedPattern = new RegExp(
      `^${nameWithoutExt.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\((\\d+)\\)${extension.replace(/\./g, '\\.')}$`
    );

    // 收集所有匹配的文件和它们的序号
    const numberedFiles = existingFiles.filter(f => numberedPattern.test(f.name));
    const usedNumbers = new Set<number>();

    // 添加原始文件名(序号0)
    usedNumbers.add(0);

    // 收集已使用的序号
    numberedFiles.forEach(f => {
      const match = f.name.match(numberedPattern);
      if (match) {
        usedNumbers.add(parseInt(match[1], 10));
      }
    });

    // 找到最小可用序号
    let newIndex = 1;
    while (usedNumbers.has(newIndex)) {
      newIndex++;
    }

    // 生成新文件名
    const newName = `${nameWithoutExt}(${newIndex})${extension}`;
    return `${basePath}/${newName}`;
  };

  /**
   * 创建新文件：在首个可写挂载目录（local/mem）下创建 untitled，成功后刷新并打开
   */
  const handleNewFile = async () => {
    // Prefer writable mounts: local or mem (skip read-only mounts like hello)
    const writableMount = files.find((n) => n.is_dir && (n.path.startsWith('/local') || n.path.startsWith('/mem')));
    if (!writableMount) {
      console.error('[handleNewFile] No writable mount found. Available files:', files);
      toast({
        title: "错误",
        description: "未找到可用的目录 (local 或 mem)",
        variant: "destructive",
      });
      return;
    }

    const basePath = writableMount.path.replace(/\/$/, '');
    const desiredName = 'untitled';

    // 生成唯一文件名(自动处理冲突)
    const uniquePath = generateUniqueFilePath(basePath, desiredName, files);
    console.log('[handleNewFile] Creating file:', { basePath, desiredName, uniquePath });

    try {
      // Use the /api/v1/fs/create endpoint which accepts JSON body
      const response = await httpFetch('/api/v1/fs/create', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: uniquePath })
      });
      const data = await response.json().catch(() => ({}));
      console.log('[handleNewFile] Response:', { status: response.status, data });
      if (!response.ok) {
        const msg = (data && (data.message || data.error)) || '创建失败';
        throw new Error(msg);
      }
      await fetchFiles('/');
      await fetchFiles(writableMount.path);

      // Open in new tab
      const node: FileNode = { path: uniquePath, name: uniquePath.split('/').pop() || 'untitled', is_dir: false };
      await handleFileSelect(node);

      toast({
        title: "文件创建成功",
        description: `已创建文件: ${uniquePath}`,
      });
    } catch (error) {
      console.error('[handleNewFile] Error:', error);
      const msg = error instanceof Error ? error.message : '创建失败';
      setOperationProblems((prev) => [...prev.slice(-2), { id: `create-${Date.now()}`, message: msg, source: '新建', severity: 'error' }]);
      toast({
        title: "创建文件失败",
        description: msg,
        variant: "destructive",
      });
    }
  };

  /**
   * 删除文件或目录（显示确认对话框）
   */
  const showDeleteDialog = () => {
    setDeleteDialogOpen(true);
  };

  const handleConfirmDelete = async () => {
    if (!contextMenu?.file) return;
    const path = contextMenu.file.path;
    const fileName = contextMenu.file.name;

    try {
      const response = await httpFetch(`/api/v1/fs/delete?path=${encodeURIComponent(path)}`, {
        method: 'DELETE'
      });
      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        const msg = (data && (data.message || data.error)) || '删除失败';
        throw new Error(msg);
      }
      setChildrenMap((prev) => {
        const next = { ...prev };
        delete next[path];
        const parent = path.includes('/') ? path.replace(/\/[^/]+$/, '') || '/' : '/';
        if (parent !== path) delete next[parent];
        return next;
      });

      // Close tab if file is open
      const tabToClose = tabs.find(t => t.path === path);
      if (tabToClose) {
        handleTabClose(tabToClose.id);
      }

      // 显示删除成功toast
      toast({
        title: "删除成功",
        description: `已删除${contextMenu.file.is_dir ? '目录' : '文件'}: ${fileName}`,
      });

      fetchFiles('/');
      handleContextMenuClose();
      setDeleteDialogOpen(false);
    } catch (error) {
      console.error('Error deleting file:', error);
      const msg = error instanceof Error ? error.message : '删除失败';
      setOperationProblems((prev) => [...prev.slice(-2), { id: `delete-${Date.now()}`, message: msg, source: '删除', severity: 'error' }]);

      // 显示删除失败toast
      toast({
        title: "删除失败",
        description: msg,
        variant: "destructive",
      });
      setDeleteDialogOpen(false);
    }
  };

  const handleDelete = async () => {
    // Show confirmation dialog
    showDeleteDialog();
  };

  const getChildren = useCallback((path: string) => childrenMap[path] ?? [], [childrenMap]);
  const handleExpandFolder = useCallback((path: string) => {
    if (childrenMap[path] !== undefined) return;
    fetchFiles(path);
  }, [childrenMap, fetchFiles]);

  const contextMenuItems = contextMenu ? (
    contextMenu.file === null
      ? [{ icon: '🔄', label: 'Refresh', onClick: handleRefresh, disabled: false }]
      : [
          { icon: '📄', label: 'Open', onClick: () => handleFileSelect(contextMenu.file!), disabled: contextMenu.file.is_dir },
          { icon: '🔄', label: 'Refresh', onClick: handleRefresh, disabled: false },
          { icon: '', label: '', onClick: () => {}, separator: true },
          { icon: '✏️', label: 'Rename', onClick: () => {}, disabled: true },
          { icon: '🗑️', label: 'Delete', onClick: handleDelete, disabled: false },
        ]
  ) : [];

  /**
   * 命令面板命令定义
   */
  const commandPaletteCommands = [
    // 文件操作
    {
      id: 'new-file',
      label: '新建文件',
      shortcut: 'Cmd+Shift+N',
      category: '文件',
      action: () => handleNewFile(),
    },
    {
      id: 'save-file',
      label: '保存文件',
      shortcut: 'Cmd+S',
      category: '文件',
      action: () => {
        if (activeTabId) handleFileSave(activeTabId);
      },
    },
    {
      id: 'close-tab',
      label: '关闭标签页',
      shortcut: 'Cmd+W',
      category: '文件',
      action: () => {
        if (activeTabId) handleTabClose(activeTabId);
      },
    },
    // 视图操作
    {
      id: 'toggle-sidebar',
      label: '切换侧边栏',
      shortcut: 'Cmd+B',
      category: '视图',
      action: () => setSidebarVisible(v => !v),
    },
    {
      id: 'show-explorer',
      label: '显示资源管理器',
      shortcut: 'Cmd+Shift+E',
      category: '视图',
      action: () => {
        setActiveView('explorer');
        setSidebarVisible(true);
      },
    },
    {
      id: 'show-terminal',
      label: '显示终端',
      shortcut: 'Cmd+J',
      category: '视图',
      action: () => {
        setActiveView('terminal');
        setPanelVisible(true);
        setPanelTab('terminal');
      },
    },
    {
      id: 'show-problems',
      label: '显示问题',
      shortcut: 'Cmd+Shift+M',
      category: '视图',
      action: () => {
        setActiveView('problems');
        setPanelVisible(true);
        setPanelTab('problems');
      },
    },
    {
      id: 'show-search',
      label: '显示搜索',
      shortcut: 'Cmd+Shift+F',
      category: '视图',
      action: () => {
        setActiveView('search');
        setSidebarVisible(true);
      },
    },
    {
      id: 'show-monitor',
      label: '显示监控',
      shortcut: 'Cmd+Shift+D',
      category: '视图',
      action: () => {
        setActiveView('monitor');
        setSidebarVisible(true);
      },
    },
    // 导航操作
    {
      id: 'quick-open',
      label: '快速打开',
      shortcut: 'Cmd+P',
      category: '导航',
      action: () => setQuickOpenOpen(true),
    },
    {
      id: 'go-to-line',
      label: '跳转到行',
      shortcut: 'Ctrl+G',
      category: '导航',
      action: () => {
        toast({
          title: "提示",
          description: "跳转到行功能将在编辑器中实现",
        });
      },
    },
    // 帮助
    {
      id: 'keyboard-shortcuts',
      label: '键盘快捷键',
      shortcut: 'Cmd+/',
      category: '帮助',
      action: () => setShortcutsOpen(true),
    },
  ];

  return (
    <div className="app">
      <NetworkBanner />
      <MenuBar
        onRefresh={handleRefresh}
        onNewFile={handleNewFile}
        newFileDisabled={files.length === 0}
        onToggleTerminal={() => {
          setPanelVisible((v) => !v);
          setActiveView('terminal');
        }}
        onToggleSidebar={() => {
          setSidebarVisible((v) => !v);
          setActiveView('explorer');
        }}
      />
      <button
        onClick={() => notificationCenter.setOpen(true)}
        className="relative p-2 hover:bg-accent rounded-md transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
        title="通知中心"
      >
        <Bell className="h-5 w-5" />
        {notificationCenter.unreadCount > 0 && (
          <span className="absolute top-1 right-1 h-5 min-w-[20px] px-1 bg-red-500 text-white text-xs font-semibold rounded-full flex items-center justify-center border-2 border-background">
            {notificationCenter.unreadCount > 9 ? '9+' : notificationCenter.unreadCount}
          </span>
        )}
      </button>

      <div className="workbench">
        <ActivityBar
          activeView={activeView}
          onViewChange={(view) => {
            setActiveView(view);
            if (view === 'explorer' || view === 'plugins' || view === 'search' || view === 'monitor') setSidebarVisible(true);
            if (view === 'terminal' || view === 'problems') {
              setPanelVisible(true);
              setPanelTab(view === 'terminal' ? 'terminal' : 'problems');
            }
          }}
          sidebarVisible={sidebarVisible}
          panelVisible={panelVisible}
          problemsCount={problems.length}
        />

        {sidebarVisible && (
          <div className="sidebar">
            {activeView === 'plugins' ? (
              <PluginManagerView />
            ) : activeView === 'search' ? (
              <SearchUploadView />
            ) : activeView === 'monitor' ? (
              <MonitorView />
            ) : activeView === 'memory' ? (
              <MemoryView />
            ) : (
            <FileTree
              files={files}
              getChildren={getChildren}
              onExpandFolder={handleExpandFolder}
              selectedFile={activeTab ? { path: activeTab.path, name: activeTab.name, is_dir: false } : null}
              onFileSelect={handleFileSelect}
              onContextMenu={handleContextMenu}
              onContextMenuEmpty={(e) => { e.preventDefault(); setContextMenu({ x: e.clientX, y: e.clientY, file: null }); }}
              loading={filesLoading}
              loadingPaths={loadingPaths}
              error={filesError}
              onRetry={handleRefresh}
            />
            )}
          </div>
        )}

        <div className="main">
          <div className="editor-container">
            {tabs.length > 0 && (
              <>
                <EditorTabs
                  tabs={tabs}
                  activeTabId={activeTabId}
                  onTabSelect={handleTabSelect}
                  onTabClose={handleTabClose}
                  onSave={handleFileSave}
                />
                {activeTab && (
                  <Breadcrumb
                    filePath={activeTab.path}
                    onNavigate={(path) => {
                      // 点击面包屑时的处理逻辑
                      // 可以扩展为在文件树中展开该路径
                      console.log('Navigate to:', path)
                    }}
                  />
                )}
                <Editor
                  file={activeTab ? { path: activeTab.path, name: activeTab.name, is_dir: false } : null}
                  content={activeTab?.content ?? ''}
                  onChange={handleEditorChange}
                  onSave={handleFileSave}
                  filesError={filesError}
                  onRetry={handleRefresh}
                  loading={isFileLoading}
                />
              </>
            )}
            {tabs.length === 0 && (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                <div className="text-center">
                  <p className="text-lg mb-2">未打开文件</p>
                  <p className="text-sm">从侧边栏选择文件以开始编辑</p>
                </div>
              </div>
            )}
          </div>
          <Panel
            activeTab={panelTab}
            onTabChange={setPanelTab}
            problems={problems}
            visible={panelVisible}
          />
        </div>
      </div>

      <StatusBar
        connected={!filesError}
        currentFilePath={activeTab?.path ?? null}
      />

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={handleContextMenuClose}
          items={contextMenuItems}
        />
      )}

      <Toaster />

      <QuickOpen
        open={quickOpenOpen}
        onClose={() => setQuickOpenOpen(false)}
        files={quickOpenFiles}
        recentFiles={recentFiles}
        onSelect={handleQuickOpenSelect}
      />

      <KeyboardShortcutsDialog
        open={shortcutsOpen}
        onClose={() => setShortcutsOpen(false)}
      />

      <CommandPalette
        open={commandPaletteOpen}
        onOpenChange={setCommandPaletteOpen}
        commands={commandPaletteCommands}
      />

      <NotificationCenter
        open={notificationCenter.open}
        onClose={() => notificationCenter.setOpen(false)}
        notifications={notificationCenter.notifications}
        onMarkRead={notificationCenter.markAsRead}
        onMarkAllRead={notificationCenter.markAllAsRead}
        onClear={notificationCenter.clearNotification}
        onClearAll={notificationCenter.clearAll}
      />

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认删除</DialogTitle>
            <DialogDescription>
              确定要删除{contextMenu?.file?.is_dir ? '目录' : '文件'} "{contextMenu?.file?.name}" 吗？
              <br />
              <span className="text-destructive">此操作无法撤销。</span>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleConfirmDelete}>
              确认删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default App;
