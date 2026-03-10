/**
 * MemoryExplorer - 记忆浏览器组件
 *
 * 展示记忆的树形结构: Category → Memory Items
 * 支持搜索过滤和点击查看详情
 * 支持多种搜索模式: vector / hybrid / llm
 * 支持创建新记忆
 */

import React, { useState, useEffect, useCallback } from 'react'
import { listCategories, getCategoryMemories, createMemory, type Category, type MemoryItem } from '@/services/memory-api'
import { searchMemories, type SearchResult } from '@/services/memory-api'
import { Skeleton, SkeletonTreeItem, SkeletonText } from '@/components/ui/skeleton'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Plus, Loader2, FileText, FolderOpen, AlertTriangle, ChevronRight, ChevronDown } from 'lucide-react'

// 搜索模式类型
type SearchMode = 'vector' | 'hybrid' | 'llm'

interface MemoryTreeProps {
  onMemorySelect?: (memory: MemoryItem) => void
  onCategorySelect?: (category: Category) => void
}

// 记忆树节点
interface TreeNode {
  id: string
  name: string
  type: 'category' | 'memory'
  data: Category | MemoryItem
  children?: TreeNode[]
}

const MemoryExplorer: React.FC<MemoryTreeProps> = ({
  onMemorySelect,
  onCategorySelect,
}) => {
  const [categories, setCategories] = useState<Category[]>([])
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(new Set())
  const [categoryMemories, setCategoryMemories] = useState<Record<string, MemoryItem[]>>({})
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResult[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [selectedMemoryId, setSelectedMemoryId] = useState<string | null>(null)

  // 搜索增强: 搜索模式
  const [searchMode, setSearchMode] = useState<SearchMode>('vector')
  const [vectorK, setVectorK] = useState<number>(10)

  // 搜索增强: 分类过滤
  const [categoryFilter, setCategoryFilter] = useState<string>('')

  // 搜索增强: 日期范围
  const [dateRange, setDateRange] = useState<{ start?: string; end?: string }>({})

  // 搜索增强: 高级搜索面板展开状态
  const [showAdvancedSearch, setShowAdvancedSearch] = useState(false)

  // 记忆创建状态
  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [newMemoryContent, setNewMemoryContent] = useState('')
  const [isCreating, setIsCreating] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)

  // 加载分类列表
  useEffect(() => {
    const fetchCategories = async () => {
      setLoading(true)
      setError(null)
      try {
        const cats = await listCategories()
        setCategories(cats)
      } catch (err) {
        // 改进错误处理：区分不同类型的错误
        let errorMessage = '无法加载记忆数据'
        if (err instanceof Error) {
          if (err.message.includes('Failed to fetch') || err.message.includes('network') || err.message.includes('Network')) {
            errorMessage = '网络连接失败，请检查后端服务是否运行'
          } else if (err.message.includes('500') || err.message.includes('Internal Server')) {
            errorMessage = '服务器内部错误，请稍后重试'
          } else if (err.message.includes('401') || err.message.includes('Unauthorized')) {
            errorMessage = '认证失败，请重新登录'
          } else if (err.message.includes('403') || err.message.includes('Forbidden')) {
            errorMessage = '没有访问权限'
          } else {
            errorMessage = `加载失败: ${err.message}`
          }
        } else {
          errorMessage = '无法加载记忆数据，请检查网络连接'
        }
        setError(errorMessage)
      } finally {
        setLoading(false)
      }
    }

    fetchCategories()
  }, [])

  // 切换分类展开状态
  const toggleCategory = useCallback(async (categoryId: string) => {
    const newExpanded = new Set(expandedCategories)
    if (newExpanded.has(categoryId)) {
      newExpanded.delete(categoryId)
    } else {
      newExpanded.add(categoryId)
      // 如果还没有加载过这个分类的记忆，则加载
      if (!categoryMemories[categoryId]) {
        try {
          const data = await getCategoryMemories(categoryId)
          setCategoryMemories(prev => ({ ...prev, [categoryId]: data.memories }))
        } catch (err) {
          console.error('Failed to load category memories:', err)
        }
      }
    }
    setExpandedCategories(newExpanded)
  }, [expandedCategories, categoryMemories])

  // 处理搜索
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) {
      setSearchResults([])
      setIsSearching(false)
      return
    }

    setIsSearching(true)
    try {
      // 调用搜索 API，传递所有搜索参数
      const results = await searchMemories(searchQuery, searchMode, vectorK)
      let filteredResults = results.results

      // 客户端分类过滤
      if (categoryFilter) {
        filteredResults = filteredResults.filter(r => r.category === categoryFilter)
      }

      // 客户端日期范围过滤 (如果后端不支持)
      if (dateRange.start || dateRange.end) {
        filteredResults = filteredResults.filter(r => {
          // 使用 created 字段进行日期过滤
          const createdDate = r.created ? new Date(r.created) : null
          if (!createdDate || isNaN(createdDate.getTime())) {
            return true // 没有日期信息的项目保留
          }

          const startDate = dateRange.start ? new Date(dateRange.start) : null
          const endDate = dateRange.end ? new Date(dateRange.end + 'T23:59:59') : null // 包含结束日期一整天

          if (startDate && createdDate < startDate) {
            return false
          }
          if (endDate && createdDate > endDate) {
            return false
          }
          return true
        })
      }

      setSearchResults(filteredResults)
    } catch (err) {
      console.error('Search failed:', err)
      setSearchResults([])
    } finally {
      setIsSearching(false)
    }
  }, [searchQuery, searchMode, vectorK, categoryFilter, dateRange])

  // 搜索防抖
  useEffect(() => {
    const timer = setTimeout(() => {
      handleSearch()
    }, 300)
    return () => clearTimeout(timer)
  }, [handleSearch])

  // 点击记忆项
  const handleMemoryClick = useCallback((memory: MemoryItem) => {
    setSelectedMemoryId(memory.id)
    onMemorySelect?.(memory)
  }, [onMemorySelect])

  // 创建新记忆
  const handleCreateMemory = useCallback(async () => {
    if (!newMemoryContent.trim()) {
      setCreateError('请输入记忆内容')
      return
    }

    setIsCreating(true)
    setCreateError(null)
    try {
      const result = await createMemory(newMemoryContent.trim())
      console.log('Memory created:', result)

      // 关闭对话框
      setShowCreateDialog(false)
      setNewMemoryContent('')

      // 刷新分类列表
      const cats = await listCategories()
      setCategories(cats)
    } catch (err) {
      let errorMessage = '创建失败'
      if (err instanceof Error) {
        if (err.message.includes('Failed to fetch') || err.message.includes('network')) {
          errorMessage = '网络连接失败，请检查后端服务'
        } else {
          errorMessage = `创建失败: ${err.message}`
        }
      }
      setCreateError(errorMessage)
    } finally {
      setIsCreating(false)
    }
  }, [newMemoryContent])

  // 渲染记忆项 - 响应式缩进
  const renderMemoryItem = (memory: MemoryItem, level: number = 1) => {
    const isSelected = selectedMemoryId === memory.id
    // 使用响应式缩进：小屏幕减少缩进量
    const indentClass = level === 1 ? '' : level === 2 ? 'pl-4 md:pl-6' : 'pl-6 md:pl-8 lg:pl-10'
    return (
      <div
        key={memory.id}
        className={`memory-item ${isSelected ? 'selected' : ''} ${indentClass}`}
        onClick={() => handleMemoryClick(memory)}
        role="button"
        tabIndex={0}
        aria-selected={isSelected}
      >
        <span className="memory-icon"><FileText className="h-3.5 w-3.5" /></span>
        <span className="memory-name truncate">{memory.summary || memory.id}</span>
        <span className="memory-type text-[10px]">{memory.type}</span>
      </div>
    )
  }

  // 渲染搜索结果
  const renderSearchResults = () => {
    if (isSearching) {
      return (
        <div className="memory-search-loading">
          <span>搜索中...</span>
        </div>
      )
    }

    if (searchResults.length === 0 && searchQuery) {
      return (
        <div className="memory-search-empty">
          <span>未找到相关记忆</span>
        </div>
      )
    }

    return searchResults.map(result => (
      <div
        key={result.id}
        className="memory-search-result"
        onClick={() => {
          const memory: MemoryItem = {
            id: result.id,
            type: result.type,
            content: result.content,
            summary: result.content.substring(0, 100),
            created: '',
            updated: '',
            category: result.category,
          }
          handleMemoryClick(memory)
        }}
        role="button"
        tabIndex={0}
      >
        <div className="search-result-header">
          <span className="memory-icon"><FileText className="h-3.5 w-3.5" /></span>
          <span className="memory-name">{result.id.substring(0, 8)}...</span>
          <span className="memory-score">{(result.score * 100).toFixed(1)}%</span>
        </div>
        <div className="search-result-content">
          {result.content.substring(0, 100)}...
        </div>
      </div>
    ))
  }

  // 加载状态 - 使用骨架屏
  if (loading) {
    return (
      <div className="memory-explorer">
        <div className="memory-explorer-header">
          <span className="header-title">MEMORY</span>
        </div>
        <div className="p-4 space-y-3">
          <Skeleton variant="rounded" height={40} className="w-full" />
          <SkeletonText height={20} className="w-full" />
          <SkeletonText height={20} className="w-3/4" />
          <div className="mt-4 space-y-2">
            <SkeletonTreeItem hasChildren />
            <SkeletonTreeItem hasChildren />
            <SkeletonTreeItem hasChildren />
          </div>
        </div>
      </div>
    )
  }

  // 错误状态 - 改进的错误提示和重试按钮
  if (error) {
    return (
      <div className="memory-explorer">
        <div className="memory-explorer-header">
          <span className="header-title">MEMORY</span>
        </div>
        <div className="memory-explorer-error">
          <div className="error-icon"><AlertTriangle className="h-5 w-5 text-destructive" /></div>
          <div className="error-message">{error}</div>
          <div className="error-actions">
            <button
              onClick={() => window.location.reload()}
              className="btn-retry"
            >
              刷新页面
            </button>
            <button
              onClick={async () => {
                setLoading(true)
                setError(null)
                try {
                  const cats = await listCategories()
                  setCategories(cats)
                } catch (err) {
                  setError(err instanceof Error ? err.message : '加载失败，请重试')
                } finally {
                  setLoading(false)
                }
              }}
              className="btn-retry primary"
            >
              重试
            </button>
          </div>
          <div className="error-hint">
            如果问题持续存在，请检查后端服务是否正常运行
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="memory-explorer">
      {/* 头部标题 */}
      <div className="memory-explorer-header flex items-center justify-between">
        <span className="header-title">MEMORY</span>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowCreateDialog(true)}
          className="h-7 px-2"
          title="创建新记忆"
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>

      {/* 搜索框 - 响应式布局 */}
      <div className="memory-search px-2 md:px-3">
        <input
          type="text"
          placeholder="搜索记忆..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="search-input w-full"
        />
        <button
          className="advanced-search-toggle flex-shrink-0"
          onClick={() => setShowAdvancedSearch(!showAdvancedSearch)}
          title="高级搜索"
        >
          {showAdvancedSearch ? '▼' : '▶'}
        </button>
      </div>

      {/* 高级搜索面板 - 响应式布局 */}
      {showAdvancedSearch && (
        <div className="memory-search-advanced px-2 md:px-3">
          <div className="search-mode-selector flex flex-col sm:flex-row sm:items-center gap-2">
            <label className="text-xs sm:text-sm whitespace-nowrap">搜索模式:</label>
            <select
              value={searchMode}
              onChange={(e) => setSearchMode(e.target.value as SearchMode)}
              className="search-mode-select w-full sm:flex-1"
            >
              <option value="vector">向量搜索 (Vector)</option>
              <option value="hybrid">混合搜索 (Hybrid)</option>
              <option value="llm">LLM 搜索 (LLM)</option>
            </select>
          </div>

          {searchMode === 'vector' && (
            <div className="search-param flex flex-col sm:flex-row sm:items-center gap-2">
              <label className="text-xs sm:text-sm whitespace-nowrap">返回数量 (K):</label>
              <input
                type="number"
                min={1}
                max={50}
                value={vectorK}
                onChange={(e) => setVectorK(parseInt(e.target.value) || 10)}
                className="search-k-input w-full sm:w-16"
              />
            </div>
          )}

          <div className="search-param flex flex-col sm:flex-row sm:items-center gap-2">
            <label className="text-xs sm:text-sm whitespace-nowrap">分类过滤:</label>
            <select
              value={categoryFilter}
              onChange={(e) => setCategoryFilter(e.target.value)}
              className="search-category-select w-full sm:flex-1"
            >
              <option value="">全部分类</option>
              {categories.map(cat => (
                <option key={cat.id} value={cat.name}>{cat.name}</option>
              ))}
            </select>
          </div>

          <div className="search-param flex flex-col gap-2">
            <label className="text-xs sm:text-sm">日期范围:</label>
            <div className="flex flex-col sm:flex-row gap-2 items-center">
              <input
                type="date"
                value={dateRange.start || ''}
                onChange={(e) => setDateRange(prev => ({ ...prev, start: e.target.value }))}
                className="search-date-input w-full"
                placeholder="开始日期"
              />
              <span className="date-separator text-xs">至</span>
              <input
                type="date"
                value={dateRange.end || ''}
                onChange={(e) => setDateRange(prev => ({ ...prev, end: e.target.value }))}
                className="search-date-input w-full"
                placeholder="结束日期"
              />
            </div>
          </div>
        </div>
      )}

      {/* 搜索结果显示区域 */}
      {searchQuery && (
        <div className="memory-search-results">
          <div className="search-results-header">
            搜索结果 ({searchResults.length})
          </div>
          {renderSearchResults()}
        </div>
      )}

      {/* 分类树形结构 */}
      {!searchQuery && (
        <div className="memory-tree">
          {categories.length === 0 ? (
            <div className="memory-tree-empty">
              <span>暂无记忆分类</span>
              <span className="hint">创建记忆后会自动生成分类</span>
            </div>
          ) : (
            categories.map(category => (
              <div key={category.id} className="memory-category">
                {/* 分类标题 */}
                <div
                  className={`memory-category-header ${expandedCategories.has(category.id) ? 'expanded' : ''}`}
                  onClick={() => toggleCategory(category.id)}
                  role="button"
                  tabIndex={0}
                >
                  <span className={`folder-icon ${expandedCategories.has(category.id) ? 'open' : ''}`}>
                    {expandedCategories.has(category.id) ? '▼' : '▶'}
                  </span>
                  <span className="category-icon"><FolderOpen className="h-3.5 w-3.5" /></span>
                  <span className="category-name">{category.name}</span>
                  <span className="category-count">{category.item_count}</span>
                </div>

                {/* 分类下的记忆列表 */}
                {expandedCategories.has(category.id) && (
                  <div className="memory-category-items">
                    {categoryMemories[category.id]?.map(memory =>
                      renderMemoryItem(memory)
                    ) ?? (
                      <div className="memory-category-loading">
                        加载中...
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* 创建记忆对话框 */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>创建新记忆</DialogTitle>
            <DialogDescription>
              输入要记忆的内容，系统将自动提取结构化信息并进行分类。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <Textarea
              placeholder="输入记忆内容...&#10;&#10;例如: 今天和 Alice 讨论了项目架构，决定采用微服务方案，使用 Rust 作为后端语言。"
              value={newMemoryContent}
              onChange={(e) => setNewMemoryContent(e.target.value)}
              className="min-h-[150px] resize-none"
              disabled={isCreating}
            />
            {createError && (
              <div className="text-sm text-destructive bg-destructive/10 rounded-md px-3 py-2">
                {createError}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowCreateDialog(false)
                setNewMemoryContent('')
                setCreateError(null)
              }}
              disabled={isCreating}
            >
              取消
            </Button>
            <Button
              onClick={handleCreateMemory}
              disabled={isCreating || !newMemoryContent.trim()}
            >
              {isCreating ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  创建中...
                </>
              ) : (
                '创建记忆'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default MemoryExplorer
