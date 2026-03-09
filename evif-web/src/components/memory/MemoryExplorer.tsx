/**
 * MemoryExplorer - 记忆浏览器组件
 *
 * 展示记忆的树形结构: Category → Memory Items
 * 支持搜索过滤和点击查看详情
 * 支持多种搜索模式: vector / hybrid / llm
 */

import React, { useState, useEffect, useCallback } from 'react'
import { listCategories, getCategoryMemories, type Category, type MemoryItem } from '@/services/memory-api'
import { searchMemories, type SearchResult } from '@/services/memory-api'

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

  // 加载分类列表
  useEffect(() => {
    const fetchCategories = async () => {
      setLoading(true)
      setError(null)
      try {
        const cats = await listCategories()
        setCategories(cats)
      } catch (err) {
        setError(err instanceof Error ? err.message : '加载分类失败')
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
          // 注意: SearchResult 没有 timestamp 字段，这里做简单处理
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

  // 渲染记忆项
  const renderMemoryItem = (memory: MemoryItem, level: number = 1) => {
    const isSelected = selectedMemoryId === memory.id
    return (
      <div
        key={memory.id}
        className={`memory-item ${isSelected ? 'selected' : ''}`}
        style={{ paddingLeft: `${level * 16 + 24}px` }}
        onClick={() => handleMemoryClick(memory)}
        role="button"
        tabIndex={0}
        aria-selected={isSelected}
      >
        <span className="memory-icon">📝</span>
        <span className="memory-name">{memory.summary || memory.id}</span>
        <span className="memory-type">{memory.type}</span>
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
          <span className="memory-icon">📝</span>
          <span className="memory-name">{result.id.substring(0, 8)}...</span>
          <span className="memory-score">{(result.score * 100).toFixed(1)}%</span>
        </div>
        <div className="search-result-content">
          {result.content.substring(0, 100)}...
        </div>
      </div>
    ))
  }

  // 加载状态
  if (loading) {
    return (
      <div className="memory-explorer">
        <div className="memory-explorer-loading">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
          <span>加载记忆数据中...</span>
        </div>
      </div>
    )
  }

  // 错误状态
  if (error) {
    return (
      <div className="memory-explorer">
        <div className="memory-explorer-error">
          <span className="text-destructive">{error}</span>
          <button onClick={() => window.location.reload()} className="btn-retry">
            重试
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="memory-explorer">
      {/* 头部标题 */}
      <div className="memory-explorer-header">
        <span className="header-title">MEMORY</span>
      </div>

      {/* 搜索框 */}
      <div className="memory-search">
        <input
          type="text"
          placeholder="搜索记忆..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="search-input"
        />
        <button
          className="advanced-search-toggle"
          onClick={() => setShowAdvancedSearch(!showAdvancedSearch)}
          title="高级搜索"
        >
          {showAdvancedSearch ? '▼' : '▶'}
        </button>
      </div>

      {/* 高级搜索面板 */}
      {showAdvancedSearch && (
        <div className="memory-search-advanced">
          <div className="search-mode-selector">
            <label>搜索模式:</label>
            <select
              value={searchMode}
              onChange={(e) => setSearchMode(e.target.value as SearchMode)}
              className="search-mode-select"
            >
              <option value="vector">向量搜索 (Vector)</option>
              <option value="hybrid">混合搜索 (Hybrid)</option>
              <option value="llm">LLM 搜索 (LLM)</option>
            </select>
          </div>

          {searchMode === 'vector' && (
            <div className="search-param">
              <label>返回数量 (K):</label>
              <input
                type="number"
                min={1}
                max={50}
                value={vectorK}
                onChange={(e) => setVectorK(parseInt(e.target.value) || 10)}
                className="search-k-input"
              />
            </div>
          )}

          <div className="search-param">
            <label>分类过滤:</label>
            <select
              value={categoryFilter}
              onChange={(e) => setCategoryFilter(e.target.value)}
              className="search-category-select"
            >
              <option value="">全部分类</option>
              {categories.map(cat => (
                <option key={cat.id} value={cat.name}>{cat.name}</option>
              ))}
            </select>
          </div>

          <div className="search-param">
            <label>日期范围:</label>
            <input
              type="date"
              value={dateRange.start || ''}
              onChange={(e) => setDateRange(prev => ({ ...prev, start: e.target.value }))}
              className="search-date-input"
              placeholder="开始日期"
            />
            <span className="date-separator">至</span>
            <input
              type="date"
              value={dateRange.end || ''}
              onChange={(e) => setDateRange(prev => ({ ...prev, end: e.target.value }))}
              className="search-date-input"
              placeholder="结束日期"
            />
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
                  <span className="category-icon">📁</span>
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
    </div>
  )
}

export default MemoryExplorer
