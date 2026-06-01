/**
 * useKeyboardShortcuts Hook
 * Manages keyboard shortcuts for the application
 * 
 * P2-2: Keyboard Shortcuts Enhancement
 */

import { useEffect, useRef, useCallback } from 'react'
import type { LegacyShortcut } from '@/lib/shortcuts'

export function useKeyboardShortcuts(shortcuts: LegacyShortcut[]) {
  const handlersRef = useRef<Map<string, () => void>>(new Map())

  const buildShortcutKey = useCallback((shortcut: LegacyShortcut): string => {
    const parts: string[] = []
    if (shortcut.ctrlKey) parts.push('ctrl')
    if (shortcut.metaKey) parts.push('meta')
    if (shortcut.shiftKey) parts.push('shift')
    if (shortcut.altKey) parts.push('alt')
    parts.push(shortcut.key.toLowerCase())
    return parts.join('+')
  }, [])

  const matchesShortcut = useCallback((event: KeyboardEvent, shortcut: LegacyShortcut): boolean => {
    const ctrlMatch = !shortcut.ctrlKey || event.ctrlKey
    const metaMatch = !shortcut.metaKey || event.metaKey
    const shiftMatch = !shortcut.shiftKey || event.shiftKey
    const altMatch = !shortcut.altKey || event.altKey
    
    const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase()
    
    if (navigator.platform.includes('Mac')) {
      return keyMatch && (shortcut.ctrlKey ? event.metaKey : true) && shiftMatch && altMatch
    }
    
    return keyMatch && ctrlMatch && shiftMatch && altMatch
  }, [])

  useEffect(() => {
    // Build handler map
    const handlerMap = new Map<string, () => void>()

    shortcuts.forEach((shortcut) => {
      const key = buildShortcutKey(shortcut)
      if (shortcut.action) {
        handlerMap.set(key, shortcut.action)
      }
    })

    handlersRef.current = handlerMap

    // Handle keyboard events
    const handleKeyDown = (event: KeyboardEvent) => {
      // Ignore if in input field
      if (
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement ||
        event.target instanceof HTMLSelectElement ||
        (event.target as HTMLElement)?.isContentEditable
      ) {
        return
      }

      // Check each shortcut
      for (const shortcut of shortcuts) {
        if (matchesShortcut(event, shortcut)) {
          event.preventDefault()
          shortcut.action?.()
          return
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [shortcuts, buildShortcutKey, matchesShortcut])

  return handlersRef.current
}
