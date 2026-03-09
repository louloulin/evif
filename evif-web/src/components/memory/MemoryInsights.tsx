/**
 * MemoryInsights - 记忆分析与洞察仪表板
 *
 * 功能:
 * - 使用统计：查询频率、热点记忆
 * - 趋势分析：记忆增长、分类分布变化
 * - 记忆健康度：陈旧度、冗余度、覆盖率
 * - 优化建议：合并建议、清理建议
 */

import React, { useState, useEffect } from 'react'
import { httpFetch } from '@/lib/http'

// 统计数据类型
interface UsageStats {
  total_memories: number
  total_categories: number
  memories_this_week: number
  queries_this_week: number
  avg_query_time_ms: number
}

// 分类分布
interface CategoryDistribution {
  category_id: string
  category_name: string
  count: number
  percentage: number
}

// 热点记忆
interface HotMemory {
  id: string
  summary: string
  type: string
  access_count: number
  last_accessed: string
}

// 健康度指标
interface HealthMetrics {
  freshness_score: number       // 新鲜度 (0-100)
  redundancy_score: number      // 冗余度 (0-100, 越低越好)
  coverage_score: number        // 覆盖率 (0-100)
  overall_health: number        // 总体健康度 (0-100)
}

// 优化建议
interface OptimizationSuggestion {
  id: string
  type: 'merge' | 'cleanup' | 'update' | 'categorize'
  priority: 'high' | 'medium' | 'low'
  title: string
  description: string
  affected_items: number
}

// 趋势数据
interface TrendData {
  date: string
  memories_created: number
  queries_made: number
}

const MemoryInsights: React.FC = () => {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // 数据状态
  const [stats, setStats] = useState<UsageStats | null>(null)
  const [categoryDistribution, setCategoryDistribution] = useState<CategoryDistribution[]>([])
  const [hotMemories, setHotMemories] = useState<HotMemory[]>([])
  const [healthMetrics, setHealthMetrics] = useState<HealthMetrics | null>(null)
  const [suggestions, setSuggestions] = useState<OptimizationSuggestion[]>([])
  const [trends, setTrends] = useState<TrendData[]>([])

  // 加载数据
  useEffect(() => {
    loadInsights()
  }, [])

  const loadInsights = async () => {
    setLoading(true)
    setError(null)

    try {
      // 并行加载所有数据
      const [statsRes, distributionRes, hotRes, healthRes, suggestionsRes, trendsRes] = await Promise.all([
        httpFetch('/api/v1/memories/stats').catch(() => null),
        httpFetch('/api/v1/memories/distribution').catch(() => null),
        httpFetch('/api/v1/memories/hot').catch(() => null),
        httpFetch('/api/v1/memories/health').catch(() => null),
        httpFetch('/api/v1/memories/suggestions').catch(() => null),
        httpFetch('/api/v1/memories/trends').catch(() => null)
      ])

      // 处理统计数据
      if (statsRes?.ok) {
        setStats(await statsRes.json())
      } else {
        // 默认数据
        setStats({
          total_memories: 0,
          total_categories: 0,
          memories_this_week: 0,
          queries_this_week: 0,
          avg_query_time_ms: 0
        })
      }

      // 处理分类分布
      if (distributionRes?.ok) {
        setCategoryDistribution(await distributionRes.json())
      } else {
        setCategoryDistribution([])
      }

      // 处理热点记忆
      if (hotRes?.ok) {
        setHotMemories(await hotRes.json())
      } else {
        setHotMemories([])
      }

      // 处理健康度
      if (healthRes?.ok) {
        setHealthMetrics(await healthRes.json())
      } else {
        setHealthMetrics({
          freshness_score: 100,
          redundancy_score: 0,
          coverage_score: 100,
          overall_health: 100
        })
      }

      // 处理建议
      if (suggestionsRes?.ok) {
        setSuggestions(await suggestionsRes.json())
      } else {
        setSuggestions([])
      }

      // 处理趋势
      if (trendsRes?.ok) {
        setTrends(await trendsRes.json())
      } else {
        setTrends([])
      }

    } catch (err) {
      console.error('Failed to load insights:', err)
      setError(err instanceof Error ? err.message : '加载洞察数据失败')
    } finally {
      setLoading(false)
    }
  }

  // 获取健康度颜色
  const getHealthColor = (score: number) => {
    if (score >= 80) return '#22c55e'
    if (score >= 60) return '#eab308'
    if (score >= 40) return '#f97316'
    return '#ef4444'
  }

  // 获取优先级样式
  const getPriorityStyle = (priority: string) => {
    switch (priority) {
      case 'high': return { background: '#fef2f2', color: '#dc2626', border: '1px solid #fecaca' }
      case 'medium': return { background: '#fffbeb', color: '#d97706', border: '1px solid #fde68a' }
      case 'low': return { background: '#f0fdf4', color: '#16a34a', border: '1px solid #bbf7d0' }
      default: return {}
    }
  }

  // 获取建议图标
  const getSuggestionIcon = (type: string) => {
    switch (type) {
      case 'merge': return '🔗'
      case 'cleanup': return '🧹'
      case 'update': return '🔄'
      case 'categorize': return '📁'
      default: return '💡'
    }
  }

  if (loading) {
    return (
      <div className="memory-insights loading">
        <div className="loading-spinner">⏳ 加载中...</div>
      </div>
    )
  }

  return (
    <div className="memory-insights">
      {/* 头部 */}
      <div className="insights-header">
        <h3>📊 记忆洞察仪表板</h3>
        <button className="refresh-button" onClick={loadInsights}>
          🔄 刷新
        </button>
      </div>

      {error && (
        <div className="insights-error">
          ⚠️ {error}
        </div>
      )}

      {/* 统计概览 */}
      {stats && (
        <div className="stats-overview">
          <div className="stat-card">
            <div className="stat-icon">📝</div>
            <div className="stat-value">{stats.total_memories}</div>
            <div className="stat-label">总记忆数</div>
          </div>
          <div className="stat-card">
            <div className="stat-icon">📁</div>
            <div className="stat-value">{stats.total_categories}</div>
            <div className="stat-label">分类数</div>
          </div>
          <div className="stat-card">
            <div className="stat-icon">📈</div>
            <div className="stat-value">{stats.memories_this_week}</div>
            <div className="stat-label">本周新增</div>
          </div>
          <div className="stat-card">
            <div className="stat-icon">🔍</div>
            <div className="stat-value">{stats.queries_this_week}</div>
            <div className="stat-label">本周查询</div>
          </div>
        </div>
      )}

      {/* 健康度指标 */}
      {healthMetrics && (
        <div className="health-section">
          <h4>💚 记忆健康度</h4>
          <div className="health-metrics">
            <div className="health-item">
              <div className="health-label">总体健康度</div>
              <div className="health-bar-container">
                <div
                  className="health-bar"
                  style={{
                    width: `${healthMetrics.overall_health}%`,
                    background: getHealthColor(healthMetrics.overall_health)
                  }}
                />
              </div>
              <div className="health-value">{healthMetrics.overall_health}%</div>
            </div>

            <div className="health-item">
              <div className="health-label">新鲜度</div>
              <div className="health-bar-container">
                <div
                  className="health-bar"
                  style={{
                    width: `${healthMetrics.freshness_score}%`,
                    background: getHealthColor(healthMetrics.freshness_score)
                  }}
                />
              </div>
              <div className="health-value">{healthMetrics.freshness_score}%</div>
            </div>

            <div className="health-item">
              <div className="health-label">冗余度</div>
              <div className="health-bar-container">
                <div
                  className="health-bar"
                  style={{
                    width: `${healthMetrics.redundancy_score}%`,
                    background: getHealthColor(100 - healthMetrics.redundancy_score)
                  }}
                />
              </div>
              <div className="health-value">{healthMetrics.redundancy_score}%</div>
            </div>

            <div className="health-item">
              <div className="health-label">覆盖率</div>
              <div className="health-bar-container">
                <div
                  className="health-bar"
                  style={{
                    width: `${healthMetrics.coverage_score}%`,
                    background: getHealthColor(healthMetrics.coverage_score)
                  }}
                />
              </div>
              <div className="health-value">{healthMetrics.coverage_score}%</div>
            </div>
          </div>
        </div>
      )}

      {/* 分类分布 */}
      {categoryDistribution.length > 0 && (
        <div className="distribution-section">
          <h4>📊 分类分布</h4>
          <div className="distribution-list">
            {categoryDistribution.map(cat => (
              <div key={cat.category_id} className="distribution-item">
                <div className="distribution-name">{cat.category_name}</div>
                <div className="distribution-bar-container">
                  <div
                    className="distribution-bar"
                    style={{ width: `${cat.percentage}%` }}
                  />
                </div>
                <div className="distribution-count">{cat.count} ({cat.percentage}%)</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 热点记忆 */}
      {hotMemories.length > 0 && (
        <div className="hot-memories-section">
          <h4>🔥 热点记忆</h4>
          <div className="hot-memories-list">
            {hotMemories.slice(0, 5).map(memory => (
              <div key={memory.id} className="hot-memory-item">
                <div className="hot-memory-rank">#{hotMemories.indexOf(memory) + 1}</div>
                <div className="hot-memory-content">
                  <div className="hot-memory-summary">{memory.summary}</div>
                  <div className="hot-memory-meta">
                    <span className="hot-memory-type">{memory.type}</span>
                    <span className="hot-memory-count">访问 {memory.access_count} 次</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 优化建议 */}
      {suggestions.length > 0 && (
        <div className="suggestions-section">
          <h4>💡 优化建议</h4>
          <div className="suggestions-list">
            {suggestions.map(suggestion => (
              <div key={suggestion.id} className="suggestion-item">
                <div className="suggestion-icon">{getSuggestionIcon(suggestion.type)}</div>
                <div className="suggestion-content">
                  <div className="suggestion-header">
                    <span className="suggestion-title">{suggestion.title}</span>
                    <span
                      className="suggestion-priority"
                      style={getPriorityStyle(suggestion.priority)}
                    >
                      {suggestion.priority === 'high' ? '高' : suggestion.priority === 'medium' ? '中' : '低'}
                    </span>
                  </div>
                  <div className="suggestion-description">{suggestion.description}</div>
                  <div className="suggestion-affected">
                    影响 {suggestion.affected_items} 个项目
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 趋势图表 (简化版) */}
      {trends.length > 0 && (
        <div className="trends-section">
          <h4>📈 趋势分析</h4>
          <div className="trends-chart">
            <div className="trends-header">
              <span>日期</span>
              <span>新增记忆</span>
              <span>查询次数</span>
            </div>
            {trends.slice(-7).map(trend => (
              <div key={trend.date} className="trends-row">
                <span>{trend.date}</span>
                <span>{trend.memories_created}</span>
                <span>{trend.queries_made}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 空状态 */}
      {!stats && !error && (
        <div className="insights-empty">
          <div className="empty-icon">📭</div>
          <p>暂无洞察数据</p>
          <p className="empty-hint">开始添加记忆后，这里将显示分析结果</p>
        </div>
      )}
    </div>
  )
}

export default MemoryInsights
