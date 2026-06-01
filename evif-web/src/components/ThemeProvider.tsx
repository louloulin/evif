/**
 * ThemeProvider - Dark/Light/System Theme Switching
 * 
 * P2-1: Theme Switching
 * - Dark/Light theme support
 * - Theme persistence to localStorage
 * - System preference sync (prefers-color-scheme)
 */

import React, { createContext, useContext, useEffect, useState, useCallback } from 'react'

export type Theme = 'dark' | 'light' | 'system'

interface ThemeProviderState {
  theme: Theme
  setTheme: (theme: Theme) => void
  resolvedTheme: 'dark' | 'light'
}

const ThemeContext = createContext<ThemeProviderState | undefined>(undefined)

const STORAGE_KEY = 'evif-theme'

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    if (typeof window === 'undefined') return 'system'
    return (localStorage.getItem(STORAGE_KEY) as Theme) || 'system'
  })

  const [resolvedTheme, setResolvedTheme] = useState<'dark' | 'light'>('dark')

  // Resolve theme based on system preference
  const resolveTheme = useCallback((t: Theme): 'dark' | 'light' => {
    if (t === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return t
  }, [])

  // Apply theme to document
  const applyTheme = useCallback((resolved: 'dark' | 'light') => {
    const root = document.documentElement
    root.classList.remove('dark', 'light')
    root.classList.add(resolved)
    setResolvedTheme(resolved)
  }, [])

  // Set theme and persist
  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme)
    localStorage.setItem(STORAGE_KEY, newTheme)
    applyTheme(resolveTheme(newTheme))
  }, [applyTheme, resolveTheme])

  // Initial theme setup and system preference listener
  useEffect(() => {
    // Apply initial theme
    applyTheme(resolveTheme(theme))

    // Listen for system preference changes
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = () => {
      if (theme === 'system') {
        applyTheme(resolveTheme('system'))
      }
    }

    mediaQuery.addEventListener('change', handleChange)
    return () => mediaQuery.removeEventListener('change', handleChange)
  }, [theme, applyTheme, resolveTheme])

  return (
    <ThemeContext.Provider value={{ theme, setTheme, resolvedTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const context = useContext(ThemeContext)
  if (!context) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return context
}

// Theme indicator component for settings UI
export function ThemeIndicator() {
  const { theme, setTheme, resolvedTheme } = useTheme()
  
  return (
    <div className="flex items-center gap-2">
      <div className="flex rounded-lg border p-1">
        <button
          onClick={() => setTheme('light')}
          className={`px-2 py-1 text-xs rounded ${
            theme === 'light' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
        >
          ☀️ Light
        </button>
        <button
          onClick={() => setTheme('dark')}
          className={`px-2 py-1 text-xs rounded ${
            theme === 'dark' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
        >
          🌙 Dark
        </button>
        <button
          onClick={() => setTheme('system')}
          className={`px-2 py-1 text-xs rounded ${
            theme === 'system' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
        >
          💻 System
        </button>
      </div>
      <span className="text-xs text-muted-foreground">
        Current: {resolvedTheme}
      </span>
    </div>
  )
}

export default ThemeProvider
