/**
 * CategoryView - 分类详情视图
 *
 * 展示分类的详细信息: 名称、描述、统计
 * 以及该分类下的记忆列表
 */

import React, { useState, useEffect, useMemo } from 'react'
import { getCategory, getCategoryMemories, type Category, type MemoryItem } from '@/services/memory-api'
import { PieChart, TrendingUp } from 'lucide-react'

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

  // 计算增强统计信息
  const stats = useMemo(() => {
    const typeDistribution: Record<string, number> = {}
    let totalChars = 0

    memories.forEach(memory => {
      // 记忆类型分布
      const type = memory.type || 'unknown'
      typeDistribution[type] = (typeDistribution[type] || 0) + 1

      // 总字符数
      totalChars += (memory.content?.length || 0) + (memory.summary?.length || 0)
    })

    // 热点记忆（最近更新的前5个）
    const hotMemories = [...memories]
      .sort((a, b) => new Date(b.updated).getTime() - new Date(a.updated).getTime())
      .slice(0, 5)

    return {
      typeDistribution,
      totalChars,
      hotMemories
    }
  }, [memories])

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

  // 格式化数字
  const formatNumber = (num: number) => {
    if (num >= 10000) return (num / 10000).toFixed(1) + '万'
    if (num >= 1000) return (num / 1000).toFixed(1) + 'k'
    return num.toString()
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

      {/* 统计信息 - 增强版 */}
      <div className="category-stats">
        <div className="stat-card">
          <span className="stat-value">{category.item_count}</span>
          <span className="stat-label">记忆总数</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{formatNumber(stats.totalChars)}</span>
          <span className="stat-label">总字符数</span>
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

      {/* 记忆类型分布 */}
      <div className="category-stats mt-4">
        <div className="stat-card flex-row items-center gap-3">
          <PieChart className="w-5 h-5 text-blue-500" />
          <div className="flex-1">
            <div className="text-xs text-muted-foreground mb-1">记忆类型分布</div>
            <div className="flex flex-wrap gap-2">
              {Object.entries(stats.typeDistribution).map(([type, count]) => (
                <span
                  key={type}
                  className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-blue-100 text-blue-700"
                >
                  {type}: {count}
                </span>
              ))}
              {Object.keys(stats.typeDistribution).length === 0 && (
                <span className="text-xs text-muted-foreground">暂无数据</span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* 热点记忆 */}
      <div className="category-stats mt-4">
        <div className="stat-card flex-row items-start gap-3">
          <TrendingUp className="w-5 h-5 text-orange-500 mt-1" />
          <div className="flex-1">
            <div className="text-xs text-muted-foreground mb-2">热点记忆 (最近更新)</div>
            <div className="space-y-1">
              {stats.hotMemories.map((memory, idx) => (
                <div
                  key={memory.id}
                  className="flex items-center gap-2 text-xs p-1.5 rounded hover:bg-muted cursor-pointer transition-colors"
                  onClick={() => onMemorySelect?.(memory)}
                >
                  <span className="text-orange-500 font-medium">{idx + 1}</span>
                  <span className="truncate flex-1">{memory.summary || memory.content.substring(0, 30)}</span>
                  <span className="text-muted-foreground">{formatDate(memory.updated)}</span>
                </div>
              ))}
              {stats.hotMemories.length === 0 && (
                <span className="text-xs text-muted-foreground">暂无热点记忆</span>
              )}
            </div>
          </div>
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
