import React, { useState, useMemo, useCallback, useEffect } from 'react'
import { Search, File, Folder, Command, Clock, FileText } from 'lucide-react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { QuickOpenItem, QuickOpenFilter } from '@/types/editor'

interface QuickOpenProps {
  open: boolean
  onClose: () => void
  files: QuickOpenItem[]
  recentFiles?: string[] // 最近打开文件的路径列表
  onSelect: (item: QuickOpenItem) => void
}

export const QuickOpen: React.FC<QuickOpenProps> = ({
  open,
  onClose,
  files,
  recentFiles = [],
  onSelect,
}) => {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [filter, setFilter] = useState<QuickOpenFilter>({})
  const [showRecentOnly, setShowRecentOnly] = useState(true)

  // 模糊搜索算法 - 更智能的匹配
  const fuzzyMatch = useCallback((text: string, query: string): boolean => {
    if (!query) return true

    const textLower = text.toLowerCase()
    const queryLower = query.toLowerCase()

    // 1. 精确匹配
    if (textLower.includes(queryLower)) return true

    // 2. 首字母匹配 (例如: "ap" 匹配 "App.tsx")
    const words = textLower.split(/[\s\-_./]/)
    const acronym = words.map(w => w[0]).join('')
    if (acronym.includes(queryLower)) return true

    // 3. 连续字符匹配 (例如: "flt" 匹配 "FiLe")
    let queryIndex = 0
    for (let i = 0; i < textLower.length && queryIndex < queryLower.length; i++) {
      if (textLower[i] === queryLower[queryIndex]) {
        queryIndex++
      }
    }
    return queryIndex === queryLower.length
  }, [])

  // 过滤文件
  const filteredFiles = useMemo(() => {
    let results = files

    // 优先显示最近打开的文件
    if (showRecentOnly && recentFiles.length > 0 && !query) {
      const recentSet = new Set(recentFiles)
      results = results.filter(f => recentSet.has(f.path))
    }

    // 类型过滤
    if (filter.type && filter.type !== 'all') {
      results = results.filter(f => f.type === filter.type)
    }

    // 语言过滤
    if (filter.language) {
      results = results.filter(f => f.language === filter.language)
    }

    // 模糊搜索查询
    if (query) {
      results = results.filter(f => {
        const nameMatch = fuzzyMatch(f.name, query)
        const pathMatch = fuzzyMatch(f.path, query)
        return nameMatch || pathMatch
      })
      // 搜索时显示所有匹配结果
    } else if (showRecentOnly && recentFiles.length > 0) {
      // 没有搜索时,按最近打开时间排序
      const recentOrder = new Map(recentFiles.map((path, idx) => [path, -idx]))
      results.sort((a, b) => {
        const orderA = recentOrder.get(a.path) ?? 999
        const orderB = recentOrder.get(b.path) ?? 999
        return orderA - orderB
      })
    }

    return results.slice(0, 20) // 限制结果数量
  }, [files, query, filter, showRecentOnly, recentFiles, fuzzyMatch])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        setSelectedIndex((prev) =>
          prev < filteredFiles.length - 1 ? prev + 1 : prev
        )
        break
      case 'ArrowUp':
        e.preventDefault()
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0))
        break
      case 'Enter':
        e.preventDefault()
        if (filteredFiles[selectedIndex]) {
          handleSelect(filteredFiles[selectedIndex])
        }
        break
      case 'Escape':
        e.preventDefault()
        handleClose()
        break
    }
  }

  const handleSelect = (item: QuickOpenItem) => {
    onSelect(item)
    handleClose()
  }

  const handleClose = () => {
    setQuery('')
    setSelectedIndex(0)
    setShowRecentOnly(true)
    onClose()
  }

  // 当对话框打开时重置状态
  useEffect(() => {
    if (open) {
      setQuery('')
      setSelectedIndex(0)
      setShowRecentOnly(true)
    }
  }, [open])

  const getFileIcon = (item: QuickOpenItem) => {
    return item.type === 'directory' ? (
      <Folder className="h-4 w-4 text-blue-500" />
    ) : (
      <File className="h-4 w-4 text-gray-500" />
    )
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Command className="h-5 w-5" />
            快速打开文件
            <span className="text-sm font-normal text-muted-foreground ml-2">
              Ctrl+P
            </span>
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* 搜索输入 */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              type="text"
              placeholder="输入文件名或路径进行搜索..."
              value={query}
              onChange={(e) => {
                setQuery(e.target.value)
                setSelectedIndex(0)
                if (e.target.value) {
                  setShowRecentOnly(false)
                }
              }}
              onKeyDown={handleKeyDown}
              className="pl-9"
              autoFocus
            />
          </div>

          {/* 最近文件提示 */}
          {!query && recentFiles.length > 0 && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Clock className="h-4 w-4" />
              <span>显示最近打开的文件</span>
            </div>
          )}

          {/* 过滤器 */}
          <div className="flex gap-2">
            <Button
              variant={!filter.type || filter.type === 'all' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter({ ...filter, type: 'all' })}
            >
              全部
            </Button>
            <Button
              variant={filter.type === 'file' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter({ ...filter, type: 'file' })}
            >
              文件
            </Button>
            <Button
              variant={filter.type === 'directory' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter({ ...filter, type: 'directory' })}
            >
              目录
            </Button>
          </div>

          {/* 文件列表 */}
          <ScrollArea className="h-[400px] border rounded-md">
            {filteredFiles.length === 0 ? (
              <div className="flex items-center justify-center h-32 text-muted-foreground">
                未找到匹配的文件
              </div>
            ) : (
              <div className="p-1">
                {filteredFiles.map((item, index) => (
                  <div
                    key={item.path}
                    className={`
                      flex items-center gap-4 px-3 py-2 rounded-md cursor-pointer
                      transition-colors
                      ${
                        index === selectedIndex
                          ? 'bg-accent'
                          : 'hover:bg-muted'
                      }
                    `}
                    onClick={() => handleSelect(item)}
                    onMouseEnter={() => setSelectedIndex(index)}
                  >
                    {/* 文件图标 */}
                    <div className="shrink-0">{getFileIcon(item)}</div>

                    {/* 最近打开标记 */}
                    {recentFiles.includes(item.path) && !query && (
                      <Clock className="h-3 w-3 text-muted-foreground" />
                    )}

                    {/* 文件信息 */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-sm truncate">
                          {item.name}
                        </span>
                        {item.language && (
                          <span className="text-xs text-muted-foreground px-1.5 py-0.5 rounded bg-muted">
                            {item.language}
                          </span>
                        )}
                      </div>
                      <div className="text-xs text-muted-foreground truncate">
                        {item.path}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </ScrollArea>

          {/* 提示信息 */}
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>
              {filteredFiles.length} 个结果
            </span>
            <div className="flex items-center gap-4">
              <span>↑↓ 导航</span>
              <span>Enter 打开</span>
              <span>Esc 关闭</span>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
