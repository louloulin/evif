/**
 * AIChatPanel - AI Memory Assistant Chat Interface
 * 
 * P1-4: AIChatPanel Enhancement
 * - Streaming response support
 * - Better markdown rendering
 * - Context preview
 */

import React, { useState, useRef, useEffect, useCallback } from 'react'
import { httpFetch } from '@/lib/http'
import {
  Search,
  BarChart3,
  RefreshCw,
  Lightbulb,
  MessageSquare,
  Bot,
  User,
  BookOpen,
  Send,
  Loader2,
  AlertTriangle,
  ChevronDown,
  ChevronUp,
  Copy,
  CheckCheck
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'

// Message type
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  relatedMemories?: RelatedMemory[]
  isStreaming?: boolean
}

// Related memory
interface RelatedMemory {
  id: string
  summary: string
  type: string
  score: number
}

// Suggested action
interface SuggestedAction {
  id: string
  label: string
  icon: string
  action: () => void
}

// API response type
interface ChatResponse {
  response: string
  related_memories?: RelatedMemory[]
  suggested_actions?: { id: string; label: string; icon: string }[]
}

// Render icon helper
const renderIcon = (iconName: string) => {
  switch (iconName) {
    case 'search': return <Search className="h-4 w-4" />
    case 'chart': return <BarChart3 className="h-4 w-4" />
    case 'refresh': return <RefreshCw className="h-4 w-4" />
    case 'lightbulb': return <Lightbulb className="h-4 w-4" />
    default: return <Search className="h-4 w-4" />
  }
}

// Simple markdown-like renderer
function renderMarkdown(text: string): JSX.Element {
  // Split by newlines and render paragraphs
  const lines = text.split('\n')
  return (
    <>
      {lines.map((line, i) => {
        // Bold text
        line = line.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        // Inline code
        line = line.replace(/`(.*?)`/g, '<code class="px-1 py-0.5 rounded bg-muted text-xs">$1</code>')
        // Links
        line = line.replace(/\[(.*?)\]\((.*?)\)/g, '<a href="$2" class="text-primary underline" target="_blank" rel="noopener">$1</a>')
        // Lists
        if (line.match(/^[-*]\s/)) {
          return <p key={i} className="pl-4 list-item list-disc" dangerouslySetInnerHTML={{ __html: line.replace(/^[-*]\s/, '') }} />
        }
        // Headers
        if (line.match(/^#\s/)) {
          return <h4 key={i} className="font-semibold mt-2" dangerouslySetInnerHTML={{ __html: line.replace(/^#\s/, '') }} />
        }
        if (line.match(/^##\s/)) {
          return <h5 key={i} className="font-medium mt-2" dangerouslySetInnerHTML={{ __html: line.replace(/^##\s/, '') }} />
        }
        // Empty lines
        if (!line.trim()) {
          return <br key={i} />
        }
        return <p key={i} dangerouslySetInnerHTML={{ __html: line }} />
      })}
    </>
  )
}

const AIChatPanel: React.FC = () => {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [input, setInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [suggestedActions, setSuggestedActions] = useState<SuggestedAction[]>([])
  const [expandedMemories, setExpandedMemories] = useState<Set<string>>(new Set())
  const [copiedId, setCopiedId] = useState<string | null>(null)

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)

  // Auto scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // Load initial suggestions
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
    } catch {
      setSuggestedActions([
        { id: '1', label: '搜索最近关于项目的记忆', icon: 'search', action: () => handleSuggestionClick('搜索最近关于项目的记忆') },
        { id: '2', label: '总结我的偏好设置', icon: 'chart', action: () => handleSuggestionClick('总结我的偏好设置') },
        { id: '3', label: '查找重复的记忆', icon: 'refresh', action: () => handleSuggestionClick('查找重复的记忆') },
        { id: '4', label: '建议需要更新的记忆', icon: 'lightbulb', action: () => handleSuggestionClick('建议需要更新的记忆') },
      ])
    }
  }

  const handleSuggestionClick = (suggestion: string) => {
    setInput(suggestion)
    inputRef.current?.focus()
  }

  // Simulate streaming response
  const simulateStreaming = async (fullResponse: string, messageId: string) => {
    const words = fullResponse.split(' ')
    let currentContent = ''
    
    for (const word of words) {
      currentContent += word + ' '
      setMessages(prev => prev.map(m => 
        m.id === messageId 
          ? { ...m, content: currentContent.trim() }
          : m
      ))
      await new Promise(resolve => setTimeout(resolve, 30)) // ~30ms per word
    }
    
    // Mark streaming complete
    setMessages(prev => prev.map(m => 
      m.id === messageId 
        ? { ...m, isStreaming: false }
        : m
    ))
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
        relatedMemories: data.related_memories,
        isStreaming: true
      }

      setMessages(prev => [...prev, assistantMessage])

      // Start streaming simulation
      simulateStreaming(data.response, assistantMessage.id)

      // Update suggested actions
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

  const toggleMemoryExpand = (id: string) => {
    setExpandedMemories(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const copyMessage = async (id: string, content: string) => {
    try {
      await navigator.clipboard.writeText(content)
      setCopiedId(id)
      setTimeout(() => setCopiedId(null), 2000)
    } catch {
      console.error('Failed to copy')
    }
  }

  return (
    <div className="ai-chat-panel flex flex-col h-full">
      {/* Header */}
      <div className="chat-header px-4 py-3 border-b flex-shrink-0">
        <h3 className="font-semibold flex items-center gap-2">
          <Bot className="h-5 w-5" />
          AI Memory Assistant
        </h3>
        <span className="text-xs text-muted-foreground">Natural language query and memory management</span>
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 min-h-0">
        <div className="chat-messages p-4 space-y-4">
          {messages.length === 0 && (
            <div className="chat-welcome text-center py-8">
              <div className="welcome-icon mb-4">
                <MessageSquare className="h-12 w-12 mx-auto text-muted-foreground/50" />
              </div>
              <h4 className="font-medium mb-2">Welcome to AI Memory Assistant</h4>
              <p className="text-sm text-muted-foreground mb-1">
                Query, organize, and manage your memories using natural language.
              </p>
              <p className="text-xs text-muted-foreground">
                Example: &quot;Find memories about API design&quot;
              </p>
            </div>
          )}

          {messages.map(message => (
            <div
              key={message.id}
              className={`chat-message ${message.role} ${message.isStreaming ? 'streaming' : ''}`}
            >
              <div className="message-header flex items-center justify-between mb-2">
                <span className="text-xs font-medium flex items-center gap-1">
                  {message.role === 'user' ? (
                    <><User className="h-3 w-3" />You</>
                  ) : (
                    <><Bot className="h-3 w-3" />Assistant</>
                  )}
                </span>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">
                    {formatTime(message.timestamp)}
                  </span>
                  {message.role === 'assistant' && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0"
                      onClick={() => copyMessage(message.id, message.content)}
                    >
                      {copiedId === message.id ? (
                        <CheckCheck className="h-3 w-3 text-green-500" />
                      ) : (
                        <Copy className="h-3 w-3" />
                      )}
                    </Button>
                  )}
                </div>
              </div>
              <div className="message-content text-sm leading-relaxed">
                {message.role === 'assistant' ? (
                  renderMarkdown(message.content)
                ) : (
                  message.content
                )}
                {message.isStreaming && (
                  <span className="inline-block w-2 h-4 ml-1 bg-primary animate-pulse" />
                )}
              </div>

              {/* Related memories */}
              {message.relatedMemories && message.relatedMemories.length > 0 && (
                <div className="related-memories mt-3 border-t pt-3">
                  <button
                    onClick={() => toggleMemoryExpand(message.id)}
                    className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
                  >
                    <BookOpen className="h-3 w-3" />
                    Related memories: {message.relatedMemories.length}
                    {expandedMemories.has(message.id) ? (
                      <ChevronUp className="h-3 w-3" />
                    ) : (
                      <ChevronDown className="h-3 w-3" />
                    )}
                  </button>
                  {expandedMemories.has(message.id) && (
                    <div className="mt-2 space-y-2">
                      {message.relatedMemories.map(memory => (
                        <div key={memory.id} className="text-xs p-2 rounded bg-muted/50">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary text-[10px]">
                              {memory.type}
                            </span>
                            <span className="text-muted-foreground">
                              {(memory.score * 100).toFixed(0)}% match
                            </span>
                          </div>
                          <p className="text-muted-foreground">{memory.summary}</p>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}

          {loading && messages[messages.length - 1]?.role === 'user' && (
            <div className="chat-message assistant">
              <div className="message-header flex items-center gap-1 mb-2">
                <Bot className="h-3 w-3" />
                <span className="text-xs">Assistant</span>
              </div>
              <div className="typing-indicator flex gap-1">
                <span className="w-2 h-2 rounded-full bg-muted-foreground/50 animate-bounce" style={{ animationDelay: '0ms' }} />
                <span className="w-2 h-2 rounded-full bg-muted-foreground/50 animate-bounce" style={{ animationDelay: '150ms' }} />
                <span className="w-2 h-2 rounded-full bg-muted-foreground/50 animate-bounce" style={{ animationDelay: '300ms' }} />
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </ScrollArea>

      {/* Suggested actions */}
      {suggestedActions.length > 0 && messages.length === 0 && (
        <div className="suggested-actions px-4 pb-2 flex-shrink-0">
          <div className="suggestions-header flex items-center gap-1 mb-2 text-xs text-muted-foreground">
            <Lightbulb className="h-3 w-3" />
            Suggested:
          </div>
          <div className="flex gap-2 flex-wrap">
            {suggestedActions.slice(0, 4).map(action => (
              <button
                key={action.id}
                className="suggestion-button text-xs px-2 py-1 rounded-full border hover:bg-muted transition-colors"
                onClick={action.action}
                disabled={loading}
              >
                {renderIcon(action.icon)}
                <span className="ml-1">{action.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Input */}
      <form className="chat-input-form px-4 py-3 border-t flex-shrink-0" onSubmit={handleSubmit}>
        <div className="flex gap-2">
          <textarea
            ref={inputRef}
            className="chat-input flex-1 resize-none rounded-md border px-3 py-2 text-sm"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask anything about your memories..."
            rows={2}
            disabled={loading}
          />
          <Button
            type="submit"
            disabled={!input.trim() || loading}
            className="self-end"
          >
            {loading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>
      </form>

      {/* Error */}
      {error && (
        <div className="chat-error px-4 py-2 bg-destructive/10 text-destructive text-sm flex items-center gap-2 flex-shrink-0">
          <AlertTriangle className="h-4 w-4" />
          {error}
        </div>
      )}
    </div>
  )
}

export default AIChatPanel
