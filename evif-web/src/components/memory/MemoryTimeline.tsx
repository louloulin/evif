/**
 * MemoryTimeline - 记忆时间线组件
 *
 * 展示记忆的时间线视图
 * 使用 Graph API 的 timeline 查询
 */

import React, { useState, useEffect, useCallback } from 'react'
import { queryGraph, type TimelineEvent } from '@/services/memory-api'

interface MemoryTimelineProps {
  categoryId?: string
  eventType?: string
  startTime?: string
  endTime?: string
}

const MemoryTimeline: React.FC<MemoryTimelineProps> = ({
  categoryId,
  eventType,
  startTime,
  endTime,
}) => {
  const [events, setEvents] = useState<TimelineEvent[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [expandedEvent, setExpandedEvent] = useState<string | null>(null)

  // 加载时间线数据
  useEffect(() => {
    const fetchTimeline = async () => {
      setLoading(true)
      setError(null)
      try {
        const response = await queryGraph('timeline', {
          category: categoryId,
          eventType: eventType,
          startTime: startTime,
          endTime: endTime,
        })
        setEvents(response.timeline || [])
      } catch (err) {
        setError(err instanceof Error ? err.message : '加载时间线失败')
      } finally {
        setLoading(false)
      }
    }

    fetchTimeline()
  }, [categoryId, eventType, startTime, endTime])

  // 切换事件展开状态
  const toggleEvent = useCallback((eventId: string) => {
    setExpandedEvent(prev => prev === eventId ? null : eventId)
  }, [])

  // 获取事件类型图标
  const getEventIcon = (eventType: string) => {
    const iconMap: Record<string, string> = {
      'created': '✨',
      'updated': '📝',
      'deleted': '🗑️',
      'merged': '🔗',
      'reinforced': '⭐',
      'decayed': '📉',
      'default': '📌',
    }
    return iconMap[eventType] || iconMap['default']
  }

  // 获取事件类型标签
  const getEventLabel = (eventType: string) => {
    const labelMap: Record<string, string> = {
      'created': '创建',
      'updated': '更新',
      'deleted': '删除',
      'merged': '合并',
      'reinforced': '强化',
      'decayed': '衰减',
    }
    return labelMap[eventType] || eventType
  }

  // 格式化时间
  const formatTime = (timestamp: string) => {
    if (!timestamp) return '-'
    const date = new Date(timestamp)
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  // 按日期分组事件
  const groupEventsByDate = () => {
    const groups: Record<string, TimelineEvent[]> = {}
    events.forEach(event => {
      const date = new Date(event.timestamp).toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      })
      if (!groups[date]) {
        groups[date] = []
      }
      groups[date].push(event)
    })
    return groups
  }

  // 加载状态
  if (loading) {
    return (
      <div className="memory-timeline">
        <div className="timeline-loading">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
          <span>加载时间线中...</span>
        </div>
      </div>
    )
  }

  // 错误状态
  if (error) {
    return (
      <div className="memory-timeline">
        <div className="timeline-error">
          <span className="text-destructive">{error}</span>
        </div>
      </div>
    )
  }

  // 空状态
  if (events.length === 0) {
    return (
      <div className="memory-timeline">
        <div className="timeline-empty">
          <span>暂无时间线数据</span>
          <span className="hint">创建记忆后会显示时间线</span>
        </div>
      </div>
    )
  }

  const groupedEvents = groupEventsByDate()

  return (
    <div className="memory-timeline">
      {/* 头部 */}
      <div className="timeline-header">
        <h3>记忆时间线</h3>
        <span className="event-count">{events.length} 个事件</span>
      </div>

      {/* 时间线内容 */}
      <div className="timeline-content">
        {Object.entries(groupedEvents).map(([date, dateEvents]) => (
          <div key={date} className="timeline-group">
            {/* 日期标记 */}
            <div className="timeline-date">
              <span className="date-line"></span>
              <span className="date-label">{date}</span>
              <span className="date-line"></span>
            </div>

            {/* 事件列表 */}
            <div className="timeline-events">
              {dateEvents.map(event => (
                <div
                  key={event.node_id}
                  className={`timeline-event ${expandedEvent === event.node_id ? 'expanded' : ''}`}
                  onClick={() => toggleEvent(event.node_id)}
                  role="button"
                  tabIndex={0}
                >
                  {/* 时间点 */}
                  <div className="event-marker">
                    <span className="event-dot"></span>
                    <span className="event-time">{formatTime(event.timestamp)}</span>
                  </div>

                  {/* 事件内容 */}
                  <div className="event-content">
                    <div className="event-header">
                      <span className="event-icon">{getEventIcon(event.event_type)}</span>
                      <span className="event-type">{getEventLabel(event.event_type)}</span>
                      <span className="event-id">{event.node_id.substring(0, 8)}...</span>
                    </div>

                    {/* 展开的详情 */}
                    {expandedEvent === event.node_id && (
                      <div className="event-details">
                        <p>记忆 ID: {event.node_id}</p>
                        <p>事件类型: {event.event_type}</p>
                        <p>时间戳: {event.timestamp}</p>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default MemoryTimeline
