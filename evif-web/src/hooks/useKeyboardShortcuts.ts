/**
 * useKeyboardShortcuts Hook
 * Manages keyboard shortcuts for the application
 */

import { useEffect, useRef } from 'react';
import { Shortcut, matchesShortcut } from '@/lib/shortcuts';

export function useKeyboardShortcuts(shortcuts: Shortcut[]) {
  const handlersRef = useRef<Map<string, () => void>>(new Map());

  useEffect(() => {
    // Build handler map
    const handlerMap = new Map<string, () => void>();

    shortcuts.forEach((shortcut) => {
      const key = buildShortcutKey(shortcut);
      handlerMap.set(key, shortcut.action);
    });

    handlersRef.current = handlerMap;

    // Handle keyboard events
    const handleKeyDown = (event: KeyboardEvent) => {
      // Ignore if in input field
      if (
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement ||
        event.target instanceof HTMLSelectElement
      ) {
        return;
      }

      // Check each shortcut
      for (const shortcut of shortcuts) {
        if (matchesShortcut(event, shortcut)) {
          event.preventDefault();
          shortcut.action();
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [shortcuts]);

  return handlersRef.current;
}

function buildShortcutKey(shortcut: Shortcut): string {
  const parts: string[] = [];
  if (shortcut.ctrlKey) parts.push('ctrl');
  if (shortcut.metaKey) parts.push('meta');
  if (shortcut.shiftKey) parts.push('shift');
  if (shortcut.altKey) parts.push('alt');
  parts.push(shortcut.key.toLowerCase());
  return parts.join('+');
}
