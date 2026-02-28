/**
 * Keyboard Shortcuts Configuration
 * Defines all keyboard shortcuts for the application
 */

export interface Shortcut {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  description: string;
  action: () => void;
}

export interface ShortcutCategory {
  name: string;
  shortcuts: Shortcut[];
}

export const DEFAULT_SHORTCUTS: ShortcutCategory[] = [
  {
    name: '文件',
    shortcuts: [
      {
        key: 'p',
        ctrlKey: true,
        metaKey: true,
        description: '快速打开',
        action: () => {}, // Will be set by implementation
      },
      {
        key: 's',
        ctrlKey: true,
        metaKey: true,
        description: '保存文件',
        action: () => {},
      },
      {
        key: 'w',
        ctrlKey: true,
        metaKey: true,
        description: '关闭标签页',
        action: () => {},
      },
      {
        key: 'n',
        ctrlKey: true,
        metaKey: true,
        shiftKey: true,
        description: '新建文件',
        action: () => {},
      },
    ],
  },
  {
    name: '导航',
    shortcuts: [
      {
        key: 'e',
        ctrlKey: true,
        shiftKey: true,
        metaKey: true,
        description: '显示资源管理器',
        action: () => {},
      },
      {
        key: '`',
        ctrlKey: true,
        metaKey: true,
        description: '显示终端',
        action: () => {},
      },
      {
        key: 'b',
        ctrlKey: true,
        shiftKey: true,
        metaKey: true,
        description: '切换侧边栏',
        action: () => {},
      },
      {
        key: 'j',
        ctrlKey: true,
        shiftKey: true,
        metaKey: true,
        description: '显示问题',
        action: () => {},
      },
    ],
  },
  {
    name: '编辑器',
    shortcuts: [
      {
        key: 'f',
        ctrlKey: true,
        metaKey: true,
        description: '查找',
        action: () => {},
      },
      {
        key: 'h',
        ctrlKey: true,
        shiftKey: true,
        metaKey: true,
        description: '替换',
        action: () => {},
      },
    ],
  },
];

/**
 * Check if event matches shortcut
 */
export function matchesShortcut(event: KeyboardEvent, shortcut: Shortcut): boolean {
  const hasCtrl = shortcut.ctrlKey || false;
  const hasMeta = shortcut.metaKey || false;
  const hasShift = shortcut.shiftKey || false;
  const hasAlt = shortcut.altKey || false;

  // Handle both Ctrl (Windows/Linux) and Meta (Mac)
  const ctrlMatch = hasCtrl ? (event.ctrlKey || event.metaKey) : !event.ctrlKey && !event.metaKey;
  const metaMatch = !hasMeta || (event.metaKey && hasMeta);
  const shiftMatch = hasShift === event.shiftKey;
  const altMatch = hasAlt === event.altKey;
  const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase();

  return ctrlMatch && metaMatch && shiftMatch && altMatch && keyMatch;
}

/**
 * Format shortcut for display
 */
export function formatShortcut(shortcut: Shortcut): string {
  const parts: string[] = [];

  if (shortcut.ctrlKey) parts.push(navigator.platform.includes('Mac') ? '⌘' : 'Ctrl');
  if (shortcut.metaKey && navigator.platform.includes('Mac')) parts.push('⌘');
  if (shortcut.shiftKey) parts.push('Shift');
  if (shortcut.altKey) parts.push('Alt');
  parts.push(shortcut.key.toUpperCase());

  return parts.join(' + ');
}
