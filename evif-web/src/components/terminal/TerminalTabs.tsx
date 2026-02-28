import React, { useState } from 'react'
import { Plus, X, Terminal, Maximize2, Minimize2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Card } from '@/components/ui/card'

export interface TerminalTab {
  id: string
  title: string
  path: string
  active?: boolean
}

interface TerminalTabsProps {
  terminals: TerminalTab[]
  activeTerminalId: string | null
  onTerminalSelect: (terminalId: string) => void
  onTerminalClose: (terminalId: string) => void
  onTerminalAdd: () => void
  onSplit?: (terminalId: string, direction: 'horizontal' | 'vertical') => void
}

export const TerminalTabs: React.FC<TerminalTabsProps> = ({
  terminals,
  activeTerminalId,
  onTerminalSelect,
  onTerminalClose,
  onTerminalAdd,
  onSplit,
}) => {
  const [hoveredTabId, setHoveredTabId] = useState<string | null>(null)

  return (
    <div className="flex flex-col h-full">
      {/* 终端标签栏 */}
      <div className="flex items-center gap-1 bg-muted/30 border-b h-9">
        <div className="flex items-center gap-1 px-1">
          {terminals.map((terminal, index) => (
            <div
              key={terminal.id}
              className={`
                group relative flex items-center gap-2 px-3 py-1 rounded-t
                border-b-2 transition-colors cursor-pointer min-w-[100px] max-w-[150px]
                ${
                  terminal.id === activeTerminalId
                    ? 'bg-background border-primary'
                    : 'bg-muted/50 border-transparent hover:bg-muted'
                }
              `}
              onClick={() => onTerminalSelect(terminal.id)}
              onMouseEnter={() => setHoveredTabId(terminal.id)}
              onMouseLeave={() => setHoveredTabId(null)}
            >
              {/* 终端图标 */}
              <Terminal className="h-3 w-3 text-muted-foreground" />

              {/* 终端名称 */}
              <span className="text-sm truncate flex-1">
                {terminal.title || `Terminal ${index + 1}`}
              </span>

              {/* 关闭按钮 */}
              {terminals.length > 1 && (
                <button
                  className={`
                    opacity-0 group-hover:opacity-100 transition-opacity
                    h-4 w-4 flex items-center justify-center rounded
                    hover:bg-muted-foreground/20
                    ${hoveredTabId === terminal.id || terminal.id === activeTerminalId ? 'opacity-100' : ''}
                  `}
                  onClick={(e) => {
                    e.stopPropagation()
                    onTerminalClose(terminal.id)
                  }}
                  title="关闭终端"
                >
                  <X className="h-3 w-3" />
                </button>
              )}
            </div>
          ))}
        </div>

        {/* 添加终端按钮 */}
        <Button
          variant="ghost"
          size="sm"
          className="h-7 w-7 p-0 shrink-0 ml-auto"
          onClick={onTerminalAdd}
          title="新建终端 (Ctrl+Shift+T)"
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>

      {/* 终端内容区域 */}
      <div className="flex-1 overflow-hidden">
        {terminals.length === 0 ? (
          <Card className="h-full flex items-center justify-center">
            <div className="text-center text-muted-foreground">
              <Terminal className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p>没有打开的终端</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-3"
                onClick={onTerminalAdd}
              >
                <Plus className="h-4 w-4 mr-2" />
                新建终端
              </Button>
            </div>
          </Card>
        ) : (
          <div className="h-full bg-black p-2 font-mono text-sm text-green-400">
            {/* 终端输出将在这里显示 */}
            {terminals.find(t => t.id === activeTerminalId) ? (
              <div className="h-full">
                <div className="opacity-70 mb-2">
                  $ Working in: {terminals.find(t => t.id === activeTerminalId)?.path || '/'}
                </div>
                <div className="flex items-center">
                  <span className="text-blue-400">$ </span>
                  <span className="ml-1 animate-pulse">_</span>
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-center h-full opacity-50">
                选择一个终端或创建新的
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
