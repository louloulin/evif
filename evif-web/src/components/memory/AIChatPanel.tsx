/**
 * AIChatPanel - AI 记忆助手对话界面
 *
 * 功能:
 * - 自然语言查询记忆
 * - AI 辅助记忆整理
 * - 上下文预览：当前对话相关的记忆
 * - 建议操作：基于意图的主动建议
 */

import React, { useState, useRef, useEffect, useCallback } from 'react'
import { httpFetch } from '@/lib/http'

// 消息类型
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  relatedMemories?: RelatedMemory[]
}

// 相关记忆
interface RelatedMemory {
  id: string
  summary: string
  type: string
  score: number
}

// 建议操作
interface SuggestedAction {
  id: string
  label: string
  icon: string
  action: () => void
}

// API 响应类型
interface ChatResponse {
  response: string
  related_memories?: RelatedMemory[]
  suggested_actions?: { id: string; label: string; icon: string }[]
}

const AIChatPanel: React.FC = () => {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [input, setInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [suggestedActions, setSuggestedActions] = useState<SuggestedAction[]>([])

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // 加载初始建议
  useEffect(() => {
    loadInitialSuggestions()
  }, [])

  const loadInitialSuggestions = async () => {
    try {
      const response = await httpFetch('/api/v1/memories/suggestions')
      if (response.ok) {
        const data = await response.json()
        if (data.suggestions) {
          setSuggestedActions(data.suggestions.map((s: { id: string; label: string; icon: string }) => ({
            id: s.id,
            label: s.label,
            icon: s.icon,
            action: () => handleSuggestionClick(s.label)
          })))
        }
      }
    } catch (err) {
      // 如果 API 不可用，使用默认建议
      setSuggestedActions([
        { id: '1', label: '搜索最近关于项目的记忆', icon: '🔍', action: () => handleSuggestionClick('搜索最近关于项目的记忆') },
        { id: '2', label: '总结我的偏好设置', icon: '📊', action: () => handleSuggestionClick('总结我的偏好设置') },
        { id: '3', label: '查找重复的记忆', icon: '🔄', action: () => handleSuggestionClick('查找重复的记忆') },
        { id: '4', label: '建议需要更新的记忆', icon: '💡', action: () => handleSuggestionClick('建议需要更新的记忆') },
      ])
    }
  }

  const handleSuggestionClick = (suggestion: string) => {
    setInput(suggestion)
    inputRef.current?.focus()
  }

  const sendMessage = useCallback(async (messageText: string) => {
    if (!messageText.trim() || loading) return

    const userMessage: ChatMessage = {
      id: `msg-${Date.now()}`,
      role: 'user',
      content: messageText.trim(),
      timestamp: new Date()
    }

    setMessages(prev => [...prev, userMessage])
    setInput('')
    setLoading(true)
    setError(null)

    try {
      const response = await httpFetch('/api/v1/memories/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: messageText.trim(),
          conversation_history: messages.map(m => ({
            role: m.role,
            content: m.content
          }))
        })
      })

      if (!response.ok) {
        throw new Error('Failed to get response from AI')
      }

      const data: ChatResponse = await response.json()

      const assistantMessage: ChatMessage = {
        id: `msg-${Date.now()}-response`,
        role: 'assistant',
        content: data.response,
        timestamp: new Date(),
        relatedMemories: data.related_memories
      }

      setMessages(prev => [...prev, assistantMessage])

      // 更新建议操作
      if (data.suggested_actions) {
        setSuggestedActions(data.suggested_actions.map(s => ({
          id: s.id,
          label: s.label,
          icon: s.icon,
          action: () => handleSuggestionClick(s.label)
        })))
      }
    } catch (err) {
      console.error('Chat error:', err)
      setError(err instanceof Error ? err.message : '发送消息失败')

      // 添加错误响应
      const errorMessage: ChatMessage = {
        id: `msg-${Date.now()}-error`,
        role: 'assistant',
        content: '抱歉，我遇到了一些问题。请确保后端服务正在运行，然后重试。',
        timestamp: new Date()
      }
      setMessages(prev => [...prev, errorMessage])
    } finally {
      setLoading(false)
    }
  }, [loading, messages])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    sendMessage(input)
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      sendMessage(input)
    }
  }

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }

  return (
    <div className="ai-chat-panel">
      {/* 头部 */}
      <div className="chat-header">
        <h3>🤖 AI 记忆助手</h3>
        <span className="chat-subtitle">自然语言查询与管理记忆</span>
      </div>

      {/* 消息区域 */}
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-welcome">
            <div className="welcome-icon">💬</div>
            <h4>欢迎使用 AI 记忆助手</h4>
            <p>您可以用自然语言查询、整理和管理您的记忆。</p>
            <p>例如："查找关于 API 设计的记忆" 或 "总结我的偏好设置"</p>
          </div>
        )}

        {messages.map(message => (
          <div
            key={message.id}
            className={`chat-message ${message.role}`}
          >
            <div className="message-header">
              <span className="message-role">
                {message.role === 'user' ? '👤 您' : '🤖 助手'}
              </span>
              <span className="message-time">{formatTime(message.timestamp)}</span>
            </div>
            <div className="message-content">
              {message.content}
            </div>

            {/* 相关记忆 */}
            {message.relatedMemories && message.relatedMemories.length > 0 && (
              <div className="related-memories">
                <div className="related-header">📚 相关记忆:</div>
                {message.relatedMemories.map(memory => (
                  <div key={memory.id} className="related-memory-item">
                    <span className="memory-type">{memory.type}</span>
                    <span className="memory-summary">{memory.summary}</span>
                    <span className="memory-score">{(memory.score * 100).toFixed(0)}%</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}

        {loading && (
          <div className="chat-message assistant loading">
            <div className="message-content">
              <span className="typing-indicator">
                <span></span>
                <span></span>
                <span></span>
              </span>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* 建议操作 */}
      {suggestedActions.length > 0 && (
        <div className="suggested-actions">
          <div className="suggestions-header">💡 建议操作:</div>
          <div className="suggestions-list">
            {suggestedActions.slice(0, 4).map(action => (
              <button
                key={action.id}
                className="suggestion-button"
                onClick={action.action}
                disabled={loading}
              >
                <span className="suggestion-icon">{action.icon}</span>
                <span className="suggestion-label">{action.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* 输入区域 */}
      <form className="chat-input-form" onSubmit={handleSubmit}>
        <textarea
          ref={inputRef}
          className="chat-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="输入您的问题或指令..."
          rows={2}
          disabled={loading}
        />
        <button
          type="submit"
          className="chat-send-button"
          disabled={!input.trim() || loading}
        >
          {loading ? '⏳' : '📤'} 发送
        </button>
      </form>

      {/* 错误提示 */}
      {error && (
        <div className="chat-error">
          ⚠️ {error}
        </div>
      )}
    </div>
  )
}

export default AIChatPanel
