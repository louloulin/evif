import React, { useState } from 'react'
import { X, Pin, RotateCw, ChevronLeft, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { EditorTab, TabAction } from '@/types/editor'

interface EditorTabsProps {
  tabs: EditorTab[]
  activeTabId: string | null
  onTabSelect: (tabId: string) => void
  onTabClose: (tabId: string) => void
  onTabCloseOthers?: (tabId: string) => void
  onTabCloseToRight?: (tabId: string) => void
  onTabCloseAll?: () => void
  onTabPin?: (tabId: string) => void
  onSave?: (tabId: string) => void
}

export const EditorTabs: React.FC<EditorTabsProps> = ({
  tabs,
  activeTabId,
  onTabSelect,
  onTabClose,
  onTabCloseOthers,
  onTabCloseToRight,
  onTabCloseAll,
  onTabPin,
  onSave,
}) => {
  const [hoveredTabId, setHoveredTabId] = useState<string | null>(null)

  const handleTabClick = (tab: EditorTab, e: React.MouseEvent) => {
    e.preventDefault()
    // 中键点击关闭标签
    if (e.button === 1) {
      onTabClose(tab.id)
      return
    }
    onTabSelect(tab.id)
  }

  const handleTabClose = (tabId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    onTabClose(tabId)
  }

  const getLanguageColor = (language: string): string => {
    const colors: Record<string, string> = {
      typescript: 'bg-blue-500',
      javascript: 'bg-yellow-500',
      python: 'bg-green-500',
      rust: 'bg-orange-500',
      go: 'bg-cyan-500',
      java: 'bg-red-500',
      json: 'bg-gray-500',
      markdown: 'bg-purple-500',
    }
    return colors[language] || 'bg-gray-400'
  }

  const activeTab = tabs.find(t => t.id === activeTabId)

  return (
    <div className="flex items-center gap-1 bg-muted/30 border-b h-9">
      {/* 标签页列表 */}
      <ScrollArea className="flex-1">
        <div className="flex items-center gap-1 px-1">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`
                group relative flex items-center gap-2 px-3 py-1 rounded-t
                border-b-2 transition-colors cursor-pointer min-w-[120px] max-w-[200px]
                ${
                  tab.id === activeTabId
                    ? 'bg-background border-primary'
                    : 'bg-muted/50 border-transparent hover:bg-muted'
                }
              `}
              onClick={(e) => handleTabClick(tab, e)}
              onMouseEnter={() => setHoveredTabId(tab.id)}
              onMouseLeave={() => setHoveredTabId(null)}
            >
              {/* 语言指示器 */}
              <div
                className={`w-2 h-2 rounded-full shrink-0 ${getLanguageColor(tab.language)}`}
              />

              {/* 文件名 */}
              <span className="text-sm truncate flex-1">
                {tab.name}
              </span>

              {/* 修改标记 */}
              {tab.modified && (
                <span className="text-yellow-500">●</span>
              )}

              {/* 固定标记 */}
              {tab.pinned && (
                <Pin className="h-3 w-3 text-muted-foreground" />
              )}

              {/* 关闭按钮 */}
              <button
                className={`
                  opacity-0 group-hover:opacity-100 transition-opacity
                  h-4 w-4 flex items-center justify-center rounded
                  hover:bg-muted-foreground/20
                  ${hoveredTabId === tab.id || tab.id === activeTabId ? 'opacity-100' : ''}
                `}
                onClick={(e) => handleTabClose(tab.id, e)}
                title="关闭标签 (Ctrl+W)"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      </ScrollArea>

      {/* 保存按钮 */}
      {activeTabId && onSave && activeTab?.modified && (
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 gap-1 shrink-0"
          onClick={() => onSave(activeTabId)}
          title="保存 (Ctrl+S)"
        >
          <RotateCw className="h-3 w-3" />
          <span className="text-xs">保存</span>
        </Button>
      )}
    </div>
  )
}
