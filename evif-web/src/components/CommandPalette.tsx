import React, { useState, useEffect, useCallback } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog';
import { Input } from './ui/input';
import { Command, CommandList, CommandGroup, CommandItem, CommandSeparator } from './ui/command';
import { cn } from '@/lib/utils';

/**
 * 命令接口定义
 */
interface CommandDefinition {
  id: string;
  label: string;
  shortcut: string;
  category: string;
  action: () => void;
}

/**
 * 命令面板组件属性
 */
interface CommandPaletteProps {
  commands?: CommandDefinition[];
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

/**
 * 默认命令列表
 * 可以通过props传入自定义命令来扩展
 */
const DEFAULT_COMMANDS: CommandDefinition[] = [
  // 文件操作
  {
    id: 'new-file',
    label: '新建文件',
    shortcut: 'Cmd+Shift+N',
    category: '文件',
    action: () => {
      // 这个action会在App.tsx中被覆盖
      console.log('New file');
    }
  },
  {
    id: 'save-file',
    label: '保存文件',
    shortcut: 'Cmd+S',
    category: '文件',
    action: () => {
      console.log('Save file');
    }
  },
  {
    id: 'close-tab',
    label: '关闭标签页',
    shortcut: 'Cmd+W',
    category: '文件',
    action: () => {
      console.log('Close tab');
    }
  },
  // 视图操作
  {
    id: 'toggle-sidebar',
    label: '切换侧边栏',
    shortcut: 'Cmd+B',
    category: '视图',
    action: () => {
      console.log('Toggle sidebar');
    }
  },
  {
    id: 'show-explorer',
    label: '显示资源管理器',
    shortcut: 'Cmd+Shift+E',
    category: '视图',
    action: () => {
      console.log('Show explorer');
    }
  },
  {
    id: 'show-terminal',
    label: '显示终端',
    shortcut: 'Cmd+J',
    category: '视图',
    action: () => {
      console.log('Show terminal');
    }
  },
  {
    id: 'show-problems',
    label: '显示问题',
    shortcut: 'Cmd+Shift+M',
    category: '视图',
    action: () => {
      console.log('Show problems');
    }
  },
  // 导航操作
  {
    id: 'quick-open',
    label: '快速打开',
    shortcut: 'Cmd+P',
    category: '导航',
    action: () => {
      console.log('Quick open');
    }
  },
  {
    id: 'go-to-line',
    label: '跳转到行',
    shortcut: 'Ctrl+G',
    category: '导航',
    action: () => {
      console.log('Go to line');
    }
  },
  // 帮助
  {
    id: 'keyboard-shortcuts',
    label: '键盘快捷键',
    shortcut: 'Cmd+/',
    category: '帮助',
    action: () => {
      console.log('Keyboard shortcuts');
    }
  },
];

/**
 * 命令面板组件
 *
 * 功能:
 * - Cmd+Shift+P 打开命令面板
 * - 模糊搜索命令
 * - 显示快捷键提示
 * - 分类显示命令(文件/编辑/视图/导航/帮助)
 * - 支持键盘导航
 */
export const CommandPalette = ({ commands = DEFAULT_COMMANDS, open, onOpenChange }: CommandPaletteProps) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const [search, setSearch] = useState('');

  // 使用受控或非受控模式
  const isOpen = open !== undefined ? open : internalOpen;
  const setIsOpen = onOpenChange || setInternalOpen;

  /**
   * 全局快捷键监听
   * Cmd+Shift+P 打开命令面板
   * Esc 关闭命令面板
   */
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+Shift+P 打开命令面板
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'P' || e.key === 'p')) {
        e.preventDefault();
        setIsOpen(true);
        return;
      }

      // Esc 关闭命令面板
      if (e.key === 'Escape' && isOpen) {
        setIsOpen(false);
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, setIsOpen]);

  /**
   * 执行命令并关闭面板
   */
  const handleCommandSelect = useCallback((command: CommandDefinition) => {
    command.action();
    setIsOpen(false);
    setSearch('');
  }, [setIsOpen]);

  /**
   * 关闭面板时清空搜索
   */
  const handleOpenChange = useCallback((newOpen: boolean) => {
    setIsOpen(newOpen);
    if (!newOpen) {
      setSearch('');
    }
  }, [setIsOpen]);

  /**
   * 过滤命令(模糊搜索)
   */
  const filteredCommands = commands.filter(cmd =>
    cmd.label.toLowerCase().includes(search.toLowerCase()) ||
    cmd.id.toLowerCase().includes(search.toLowerCase())
  );

  /**
   * 按分类分组命令
   */
  const groupedCommands = filteredCommands.reduce((acc, cmd) => {
    if (!acc[cmd.category]) {
      acc[cmd.category] = [];
    }
    acc[cmd.category].push(cmd);
    return acc;
  }, {} as Record<string, CommandDefinition[]>);

  /**
   * 分类顺序定义
   */
  const categoryOrder = ['文件', '编辑', '视图', '导航', '帮助'];

  return (
    <Dialog open={isOpen} onOpenChange={handleOpenChange}>
      <DialogContent className="p-0 shadow-lg max-w-2xl">
        <DialogHeader className="sr-only">
          <DialogTitle>命令面板</DialogTitle>
        </DialogHeader>
        <Command className="border-0">
          {/* 搜索输入框 */}
          <div className="flex items-center border-b px-3">
            <div className="flex-1">
              <Input
                placeholder="输入命令或搜索..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="border-0 focus-visible:ring-0 focus-visible:ring-offset-0 text-base"
                autoFocus
              />
            </div>
          </div>

          <CommandList>
            {search === '' ? (
              <>
                {/* 无搜索时显示所有分类 */}
                {categoryOrder.map((category, idx) => {
                  const categoryCommands = groupedCommands[category];
                  if (!categoryCommands || categoryCommands.length === 0) return null;

                  return (
                    <React.Fragment key={category}>
                      {idx > 0 && <CommandSeparator />}
                      <CommandGroup heading={category}>
                        {categoryCommands.map((cmd) => (
                          <CommandItem
                            key={cmd.id}
                            onSelect={() => handleCommandSelect(cmd)}
                            className="flex justify-between items-center"
                          >
                            <span>{cmd.label}</span>
                            <span className="text-xs text-muted-foreground ml-2">
                              {cmd.shortcut}
                            </span>
                          </CommandItem>
                        ))}
                      </CommandGroup>
                    </React.Fragment>
                  );
                })}
              </>
            ) : (
              <>
                {/* 有搜索时显示匹配结果 */}
                {Object.entries(groupedCommands).map(([category, categoryCommands], idx) => (
                  <React.Fragment key={category}>
                    {idx > 0 && <CommandSeparator />}
                    <CommandGroup heading={category}>
                      {categoryCommands.map((cmd) => (
                        <CommandItem
                          key={cmd.id}
                          onSelect={() => handleCommandSelect(cmd)}
                          className="flex justify-between items-center"
                        >
                          <span>{cmd.label}</span>
                          <span className="text-xs text-muted-foreground ml-2">
                            {cmd.shortcut}
                          </span>
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  </React.Fragment>
                ))}
              </>
            )}

            {filteredCommands.length === 0 && (
              <div className="py-6 text-center text-sm text-muted-foreground">
                没有找到匹配的命令
              </div>
            )}
          </CommandList>
        </Command>

        {/* 底部提示 */}
        <div className="border-t px-3 py-2 text-xs text-muted-foreground flex justify-between">
          <div>
            <span className="inline-flex items-center gap-2">
              <kbd className="px-1.5 py-0.5 rounded bg-muted text-xs">↑↓</kbd>
              <span>导航</span>
            </span>
            <span className="mx-2">•</span>
            <span className="inline-flex items-center gap-2">
              <kbd className="px-1.5 py-0.5 rounded bg-muted text-xs">Enter</kbd>
              <span>执行</span>
            </span>
            <span className="mx-2">•</span>
            <span className="inline-flex items-center gap-2">
              <kbd className="px-1.5 py-0.5 rounded bg-muted text-xs">Esc</kbd>
              <span>关闭</span>
            </span>
          </div>
          <div>
            <span className="inline-flex items-center gap-2">
              <kbd className="px-1.5 py-0.5 rounded bg-muted text-xs">Cmd+Shift+P</kbd>
              <span>打开命令面板</span>
            </span>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default CommandPalette;
