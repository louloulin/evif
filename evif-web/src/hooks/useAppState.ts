/**
 * App State Hook - 集中管理应用全局状态
 * 从 App.tsx 提取状态管理逻辑
 */

import { useState, useCallback, useMemo } from 'react'

// 简化类型定义，避免循环依赖
export interface FileNode {
  id: string
  name: string
  path: string
  isDirectory: boolean
  children?: FileNode[]
}

export interface EditorTab {
  id: string
  path: string
  name: string
  content?: string
  isDirty?: boolean
}

export interface PanelTab {
  name: string
  id: string
}

export interface ProblemItem {
  id: string
  message: string
  severity: 'error' | 'warning' | 'info'
  file?: string
  line?: number
}

export interface ContextMenuPosition {
  x: number
  y: number
}

export type ActivityView = 'explorer' | 'search' | 'memory' | 'plugins' | 'monitor' | 'admin' | 'billing'

export interface AppState {
  // File Tree State
  files: FileNode[]
  setFiles: React.Dispatch<React.SetStateAction<FileNode[]>>
  childrenMap: Record<string, FileNode[]>
  setChildrenMap: React.Dispatch<React.SetStateAction<Record<string, FileNode[]>>>
  selectedPath: string | null
  setSelectedPath: (path: string | null) => void
  expandedFolders: Set<string>
  toggleFolder: (path: string) => void
  filesError: string | null
  
  // Editor State
  tabs: EditorTab[]
  setTabs: React.Dispatch<React.SetStateAction<EditorTab[]>>
  activeTabId: string | null
  setActiveTabId: (id: string | null) => void
  
  // Panel State
  panelTab: PanelTab
  setPanelTab: (tab: PanelTab) => void
  operationProblems: ProblemItem[]
  setOperationProblems: React.Dispatch<React.SetStateAction<ProblemItem[]>>
  
  // Activity State
  activeView: ActivityView | null
  setActiveView: (view: ActivityView | null) => void
  
  // Context Menu State
  contextMenu: ContextMenuPosition | null
  setContextMenu: (menu: ContextMenuPosition | null) => void
  
  // UI State
  showCommandPalette: boolean
  setShowCommandPalette: (show: boolean) => void
  showKeyboardShortcuts: boolean
  setShowKeyboardShortcuts: (show: boolean) => void
  showQuickOpen: boolean
  setShowQuickOpen: (show: boolean) => void
  
  // Recent Files
  recentFiles: string[]
  setRecentFiles: React.Dispatch<React.SetStateAction<string[]>>
  addRecentFile: (path: string) => void
  
  // Notification
  notificationCount: number
  
  // Modal State
  currentModal: string | null
  setCurrentModal: (modal: string | null) => void
}

export function useAppState(): AppState {
  // ============ File Tree State ============
  const [files, setFiles] = useState<FileNode[]>([])
  const [childrenMap, setChildrenMap] = useState<Record<string, FileNode[]>>({})
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set())
  const [filesError, setFilesError] = useState<string | null>(null)

  const toggleFolder = useCallback((path: string) => {
    setExpandedFolders(prev => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }, [])

  // ============ Editor State ============
  const [tabs, setTabs] = useState<EditorTab[]>([])
  const [activeTabId, setActiveTabId] = useState<string | null>(null)

  // ============ Panel State ============
  const [panelTab, setPanelTab] = useState<PanelTab>({ name: 'Terminal', id: 'terminal' })
  const [operationProblems, setOperationProblems] = useState<ProblemItem[]>([])

  // ============ Activity State ============
  const [activeView, setActiveView] = useState<ActivityView | null>('explorer')

  // ============ Context Menu State ============
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null)

  // ============ UI State ============
  const [showCommandPalette, setShowCommandPalette] = useState(false)
  const [showKeyboardShortcuts, setShowKeyboardShortcuts] = useState(false)
  const [showQuickOpen, setShowQuickOpen] = useState(false)

  // ============ Recent Files ============
  const [recentFiles, setRecentFiles] = useState<string[]>([])

  const addRecentFile = useCallback((path: string) => {
    setRecentFiles(prev => {
      const filtered = prev.filter(p => p !== path)
      return [path, ...filtered].slice(0, 10)
    })
  }, [])

  // ============ Notification ============
  const [notificationCount] = useState(0)

  // ============ Modal State ============
  const [currentModal, setCurrentModal] = useState<string | null>(null)

  return useMemo(() => ({
    // File Tree State
    files, setFiles,
    childrenMap, setChildrenMap,
    selectedPath, setSelectedPath,
    expandedFolders, toggleFolder,
    filesError,
    
    // Editor State
    tabs, setTabs,
    activeTabId, setActiveTabId,
    
    // Panel State
    panelTab, setPanelTab,
    operationProblems, setOperationProblems,
    
    // Activity State
    activeView, setActiveView,
    
    // Context Menu State
    contextMenu, setContextMenu,
    
    // UI State
    showCommandPalette, setShowCommandPalette,
    showKeyboardShortcuts, setShowKeyboardShortcuts,
    showQuickOpen, setShowQuickOpen,
    
    // Recent Files
    recentFiles, setRecentFiles,
    addRecentFile,
    
    // Notification
    notificationCount,
    
    // Modal State
    currentModal, setCurrentModal,
  }), [
    files, childrenMap, selectedPath, expandedFolders, filesError,
    tabs, activeTabId, panelTab, operationProblems, activeView, contextMenu,
    showCommandPalette, showKeyboardShortcuts, showQuickOpen,
    recentFiles, notificationCount, currentModal, toggleFolder, addRecentFile,
  ])
}

export default useAppState
