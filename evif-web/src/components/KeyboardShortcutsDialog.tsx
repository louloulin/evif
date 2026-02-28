/**
 * Keyboard Shortcuts Dialog
 * Displays all available keyboard shortcuts
 */

import React from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { DEFAULT_SHORTCUTS, formatShortcut } from '@/lib/shortcuts';

interface KeyboardShortcutsDialogProps {
  open: boolean;
  onClose: () => void;
}

export const KeyboardShortcutsDialog: React.FC<KeyboardShortcutsDialogProps> = ({
  open,
  onClose,
}) => {
  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>键盘快捷键</DialogTitle>
        </DialogHeader>

        <ScrollArea className="h-[500px]">
          <div className="space-y-6 p-4">
            {DEFAULT_SHORTCUTS.map((category) => (
              <div key={category.name}>
                <h3 className="text-lg font-semibold mb-3">{category.name}</h3>
                <div className="space-y-2">
                  {category.shortcuts.map((shortcut, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between p-2 rounded-md hover:bg-muted"
                    >
                      <span className="text-sm">{shortcut.description}</span>
                      <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-300 rounded-md">
                        {formatShortcut(shortcut)}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>

        <div className="text-xs text-muted-foreground text-center p-2">
          按 <kbd className="px-1 py-0.5 bg-gray-100 border rounded">Esc</kbd> 关闭
        </div>
      </DialogContent>
    </Dialog>
  );
};
