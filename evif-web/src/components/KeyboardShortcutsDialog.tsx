/**
 * Keyboard Shortcuts Dialog
 * 
 * P2-2: Keyboard Shortcuts Enhancement
 * - Global shortcuts display
 * - Category grouping
 * - Platform-aware display
 */

import React from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { DEFAULT_SHORTCUTS, formatShortcut, getShortcutDisplay, type ShortcutCategory } from '@/lib/shortcuts'
import { isMac } from '@/lib/utils'

interface KeyboardShortcutsDialogProps {
  open: boolean
  onClose: () => void
}

export const KeyboardShortcutsDialog: React.FC<KeyboardShortcutsDialogProps> = ({
  open,
  onClose,
}) => {
  const platform = isMac ? 'macOS' : 'Windows/Linux'
  
  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <span>⌨️ 键盘快捷键</span>
            <span className="text-xs font-normal text-muted-foreground">({platform})</span>
          </DialogTitle>
        </DialogHeader>

        <ScrollArea className="h-[500px]">
          <div className="space-y-6 p-4">
            {DEFAULT_SHORTCUTS.map((category: ShortcutCategory) => (
              <div key={category.name}>
                <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wider">
                  {category.name}
                </h3>
                <div className="space-y-1">
                  {category.shortcuts.map((shortcut, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-muted/50 transition-colors group"
                    >
                      <span className="text-sm group-hover:text-foreground">
                        {shortcut.description}
                      </span>
                      <kbd className="px-2 py-1 text-xs font-mono bg-muted border border-border rounded-md group-hover:border-primary/50 transition-colors">
                        {getShortcutDisplay(shortcut)}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>

        <div className="text-xs text-muted-foreground text-center p-2 border-t">
          按 <kbd className="px-1.5 py-0.5 bg-muted border rounded mx-1">Esc</kbd> 关闭
          &nbsp;•&nbsp;
          按 <kbd className="px-1.5 py-0.5 bg-muted border rounded mx-1">?</kbd> 打开此对话框
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default KeyboardShortcutsDialog
