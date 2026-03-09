/**
 * CategoryView - 分类详情视图
 *
 * 展示分类的详细信息: 名称、描述、统计
 * 以及该分类下的记忆列表
 */

import React, { useState, useEffect } from 'react'
import { getCategory, getCategoryMemories, type Category, type MemoryItem } from '@/services/memory-api'

interface CategoryViewProps {
  categoryId: string
  onBack?: () => void
  onMemorySelect?: (memory: MemoryItem) => void
}

const CategoryView: React.FC<CategoryViewProps> = ({
  categoryId,
  onBack,
  onMemorySelect,
}) => {
  const [category, setCategory] = useState<Category | null>(null)
  const [memories, setMemories] = useState<MemoryItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [sortBy, setSortBy] = useState<'created' | 'updated'>('updated')

  // 加载分类和记忆
  useEffect(() => {
    const fetchData = async () => {
      setLoading(true)
      setError(null)
      try {
        const [catData, memData] = await Promise.all([
          getCategory(categoryId),
          getCategoryMemories(categoryId),
        ])
        setCategory(catData)
        setMemories(memData.memories)
      } catch (err) {
        setError(err instanceof Error ? err.message : '加载分类详情失败')
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [categoryId])

  // 排序记忆
  const sortedMemories = [...memories].sort((a, b) => {
    if (sortBy === 'created') {
      return new Date(b.created).getTime() - new Date(a.created).getTime()
    } else {
      return new Date(b.updated).getTime() - new Date(a.updated).getTime()
    }
  })

  // 格式化日期
  const formatDate = (dateStr: string) => {
    if (!dateStr) return '-'
    const date = new Date(dateStr)
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  // 加载状态
  if (loading) {
    return (
      <div className="category-view">
        <div className="category-view-loading">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
          <span>加载分类详情中...</span>
        </div>
      </div>
    )
  }

  // 错误状态
  if (error || !category) {
    return (
      <div className="category-view">
        <div className="category-view-error">
          <span className="text-destructive">{error || '分类不存在'}</span>
          {onBack && (
            <button onClick={onBack} className="btn-back">
              返回
            </button>
          )}
        </div>
      </div>
    )
  }

  return (
    <div className="category-view">
      {/* 头部 */}
      <div className="category-view-header">
        {onBack && (
          <button onClick={onBack} className="btn-back" aria-label="返回">
            ← 返回
          </button>
        )}
        <div className="category-info">
          <h2 className="category-title">{category.name}</h2>
          <p className="category-description">{category.description || '暂无描述'}</p>
        </div>
      </div>

      {/* 统计信息 */}
      <div className="category-stats">
        <div className="stat-card">
          <span className="stat-value">{category.item_count}</span>
          <span className="stat-label">记忆总数</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{formatDate(category.created)}</span>
          <span className="stat-label">创建时间</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{formatDate(category.updated)}</span>
          <span className="stat-label">更新时间</span>
        </div>
      </div>

      {/* 记忆列表 */}
      <div className="category-memories">
        <div className="memories-header">
          <h3>记忆列表</h3>
          <div className="sort-controls">
            <label>排序:</label>
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as 'created' | 'updated')}
              className="sort-select"
            >
              <option value="updated">按更新时间</option>
              <option value="created">按创建时间</option>
            </select>
          </div>
        </div>

        {sortedMemories.length === 0 ? (
          <div className="memories-empty">
            <span>该分类下暂无记忆</span>
          </div>
        ) : (
          <div className="memories-list">
            {sortedMemories.map(memory => (
              <div
                key={memory.id}
                className="memory-item-card"
                onClick={() => onMemorySelect?.(memory)}
                role="button"
                tabIndex={0}
              >
                <div className="memory-item-header">
                  <span className="memory-type-badge">{memory.type}</span>
                  <span className="memory-id">{memory.id.substring(0, 8)}...</span>
                </div>
                <div className="memory-item-summary">
                  {memory.summary || memory.content.substring(0, 100)}
                </div>
                <div className="memory-item-meta">
                  <span>创建: {formatDate(memory.created)}</span>
                  <span>更新: {formatDate(memory.updated)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

export default CategoryView
