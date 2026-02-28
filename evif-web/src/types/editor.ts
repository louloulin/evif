// 编辑器相关类型定义

export interface EditorTab {
  id: string
  path: string
  name: string
  content: string
  language: string
  modified?: boolean
  active?: boolean
  pinned?: boolean
}

export type TabAction = 'close' | 'close-others' | 'close-to-right' | 'close-all' | 'pin'

export interface EditorPosition {
  lineNumber: number
  column: number
}

export interface QuickOpenItem {
  path: string
  name: string
  type: 'file' | 'directory'
  language?: string
}

export interface QuickOpenFilter {
  type?: 'file' | 'directory' | 'all'
  language?: string
  searchPath?: string
}
