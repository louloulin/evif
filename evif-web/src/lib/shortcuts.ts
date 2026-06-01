/**
 * Keyboard Shortcuts Configuration
 * 
 * P2-2: Keyboard Shortcuts Enhancement
 * - Global shortcuts
 * - Custom bindings
 * - Shortcut hints
 */

export interface Shortcut {
  key: string
  ctrlKey?: boolean
  metaKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  description: string
  category: string
  action?: () => void  // Optional action for useKeyboardShortcuts hook
}

export interface ShortcutCategory {
  name: string
  shortcuts: Omit<Shortcut, 'action'>[]
}

export interface ShortcutDefinition {
  name: string
  shortcuts: Shortcut[]
}

// Legacy interface for backward compatibility
export interface LegacyShortcut {
  key: string
  ctrlKey?: boolean
  metaKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  description: string
  action: () => void
}

export const DEFAULT_SHORTCUTS: ShortcutDefinition[] = [
  {
    name: '文件操作',
    shortcuts: [
      { key: 'p', ctrlKey: true, metaKey: true, description: '快速打开文件', category: 'file' },
      { key: 's', ctrlKey: true, metaKey: true, description: '保存文件', category: 'file' },
      { key: 'w', ctrlKey: true, metaKey: true, description: '关闭标签页', category: 'file' },
      { key: 'n', ctrlKey: true, metaKey: true, shiftKey: true, description: '新建文件', category: 'file' },
      { key: 'o', ctrlKey: true, metaKey: true, description: '打开文件', category: 'file' },
    ],
  },
  {
    name: '编辑操作',
    shortcuts: [
      { key: 'z', ctrlKey: true, metaKey: true, description: '撤销', category: 'edit' },
      { key: 'z', ctrlKey: true, metaKey: true, shiftKey: true, description: '重做', category: 'edit' },
      { key: 'a', ctrlKey: true, metaKey: true, description: '全选', category: 'edit' },
      { key: 'd', ctrlKey: true, metaKey: true, description: '复制行', category: 'edit' },
      { key: 'x', ctrlKey: true, metaKey: true, description: '剪切', category: 'edit' },
      { key: 'c', ctrlKey: true, metaKey: true, description: '复制', category: 'edit' },
      { key: 'v', ctrlKey: true, metaKey: true, description: '粘贴', category: 'edit' },
    ],
  },
  {
    name: '视图切换',
    shortcuts: [
      { key: 'e', ctrlKey: true, metaKey: true, shiftKey: true, description: '资源管理器', category: 'view' },
      { key: '`', ctrlKey: true, metaKey: true, description: '终端', category: 'view' },
      { key: 'b', ctrlKey: true, metaKey: true, shiftKey: true, description: '切换侧边栏', category: 'view' },
      { key: 'j', ctrlKey: true, metaKey: true, shiftKey: true, description: '问题面板', category: 'view' },
      { key: 't', ctrlKey: true, metaKey: true, description: '新标签页', category: 'view' },
      { key: 'Tab', altKey: true, description: '切换标签', category: 'view' },
    ],
  },
  {
    name: '搜索',
    shortcuts: [
      { key: 'f', ctrlKey: true, metaKey: true, description: '查找', category: 'search' },
      { key: 'f', ctrlKey: true, metaKey: true, shiftKey: true, description: '替换', category: 'search' },
      { key: 'h', ctrlKey: true, metaKey: true, description: '文件内搜索', category: 'search' },
    ],
  },
  {
    name: '命令',
    shortcuts: [
      { key: 'p', ctrlKey: true, metaKey: true, shiftKey: true, description: '命令面板', category: 'command' },
      { key: '/', ctrlKey: true, metaKey: true, shiftKey: true, description: '快捷键参考', category: 'command' },
    ],
  },
  {
    name: '记忆',
    shortcuts: [
      { key: 'm', ctrlKey: true, metaKey: true, description: '记忆视图', category: 'memory' },
      { key: 'r', ctrlKey: true, metaKey: true, description: '刷新', category: 'memory' },
    ],
  },
]

/**
 * Format shortcut for display
 */
export function formatShortcut(shortcut: Shortcut | LegacyShortcut): string {
  const parts: string[] = []
  
  if (shortcut.ctrlKey || shortcut.metaKey) {
    parts.push(navigator.platform.includes('Mac') ? '⌘' : 'Ctrl')
  }
  if (shortcut.altKey) {
    parts.push(navigator.platform.includes('Mac') ? '⌥' : 'Alt')
  }
  if (shortcut.shiftKey) {
    parts.push(navigator.platform.includes('Mac') ? '⇧' : 'Shift')
  }
  
  // Format key name
  let key = shortcut.key
  if (key === 'ArrowUp') key = '↑'
  else if (key === 'ArrowDown') key = '↓'
  else if (key === 'ArrowLeft') key = '←'
  else if (key === 'ArrowRight') key = '→'
  else if (key === ' ') key = 'Space'
  else if (key === '`') key = '`'
  
  parts.push(key.toUpperCase())
  
  return parts.join(' + ')
}

/**
 * Check if keyboard event matches shortcut
 */
export function matchesShortcut(event: KeyboardEvent, shortcut: Shortcut | LegacyShortcut): boolean {
  const ctrlMatch = !shortcut.ctrlKey || event.ctrlKey
  const metaMatch = !shortcut.metaKey || event.metaKey
  const shiftMatch = !shortcut.shiftKey || event.shiftKey
  const altMatch = !shortcut.altKey || event.altKey
  
  const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase()
  
  if (navigator.platform.includes('Mac')) {
    return keyMatch && (shortcut.ctrlKey ? event.metaKey : true) && shiftMatch && altMatch
  }
  
  return keyMatch && ctrlMatch && shiftMatch && altMatch
}

/**
 * Get shortcut display key for a specific platform
 */
export function getShortcutDisplay(shortcut: Shortcut | LegacyShortcut): string {
  const isMac = navigator.platform.includes('Mac')
  
  if (isMac) {
    const parts: string[] = []
    if (shortcut.ctrlKey) parts.push('⌃')
    if (shortcut.metaKey) parts.push('⌘')
    if (shortcut.shiftKey) parts.push('⇧')
    if (shortcut.altKey) parts.push('⌥')
    parts.push(shortcut.key.toUpperCase())
    return parts.join('')
  }
  
  return formatShortcut(shortcut)
}
