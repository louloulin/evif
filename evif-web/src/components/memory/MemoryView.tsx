/**
 * MemoryView - 记忆管理视图
 * 核心闭环: 创建记忆 → 分类管理 → 搜索检索 → 关系分析
 */

import React, { useState, useEffect } from 'react'
import MemoryExplorer from './MemoryExplorer'
import CategoryView from './CategoryView'
import MemoryTimeline from './MemoryTimeline'
import KnowledgeGraph from './KnowledgeGraph'
import AIChatPanel from './AIChatPanel'
import MemoryInsights from './MemoryInsights'
import { LoadingSpinner, EmptyState, ErrorState } from '@/components/ui/loading'
import { listMemories, listCategories, MemoryItem, Category } from '@/services/memory-api'

type MemoryViewTab = 'explorer' | 'timeline' | 'graph' | 'ai-chat' | 'insights'

const MemoryView: React.FC = () => {
  // ============ State ============
  const [activeTab, setActiveTab] = useState<MemoryViewTab>('explorer')
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null)
  const [selectedMemory, setSelectedMemory] = useState<MemoryItem | null>(null)
  
  // Core Data States
  const [memories, setMemories] = useState<MemoryItem[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  
  // Loading & Error States
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // ============ Data Fetching ============
  const fetchData = async () => {
    setLoading(true)
    setError(null)
    try {
      const [memoriesData, categoriesData] = await Promise.all([
        listMemories(),
        listCategories(),
      ])
      setMemories(memoriesData)
      setCategories(categoriesData)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load memories')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
  }, [])

  // ============ Handlers ============
  const handleCategorySelect = (category: Category) => {
    setSelectedCategoryId(category.id)
  }

  const handleMemorySelect = (memory: MemoryItem) => {
    setSelectedMemory(memory)
  }

  const handleBackToCategories = () => {
    setSelectedCategoryId(null)
  }

  // ============ Loading State ============
  if (loading) {
    return (
      <div className="memory-view h-full flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <LoadingSpinner size="lg" />
          <p className="text-sm text-muted-foreground">Loading memories...</p>
        </div>
      </div>
    )
  }

  // ============ Error State ============
  if (error) {
    return (
      <div className="memory-view h-full">
        <ErrorState error={error} onRetry={fetchData} />
      </div>
    )
  }

  // ============ Empty State ============
  if (memories.length === 0 && categories.length === 0) {
    return (
      <div className="memory-view h-full">
        <EmptyState
          icon="folder"
          title="No memories yet"
          description="Start by creating your first memory to capture important information."
          action={{
            label: 'Create Memory',
            onClick: () => console.log('Create memory'),
          }}
        />
      </div>
    )
  }

  // ============ Category Detail View ============
  if (selectedCategoryId) {
    return (
      <CategoryView
        categoryId={selectedCategoryId}
        onBack={handleBackToCategories}
        onMemorySelect={handleMemorySelect}
      />
    )
  }

  // ============ Main View ============
  return (
    <div className="memory-view flex flex-col h-full">
      {/* Tab Navigation */}
      <div className="memory-view-tabs border-b flex">
        {[
          { id: 'explorer', label: '浏览器', icon: '📁' },
          { id: 'timeline', label: '时间线', icon: '📅' },
          { id: 'graph', label: '关系图', icon: '🔗' },
          { id: 'ai-chat', label: 'AI 助手', icon: '🤖' },
          { id: 'insights', label: '洞察', icon: '💡' },
        ].map((tab) => (
          <button
            key={tab.id}
            className={`px-4 py-2 text-sm font-medium transition-colors relative ${
              activeTab === tab.id
                ? 'text-primary'
                : 'text-muted-foreground hover:text-foreground'
            }`}
            onClick={() => setActiveTab(tab.id as MemoryViewTab)}
          >
            <span className="mr-2">{tab.icon}</span>
            {tab.label}
            {activeTab === tab.id && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="memory-view-content flex-1 overflow-auto">
        {activeTab === 'explorer' && (
          <MemoryExplorer
            memories={memories}
            categories={categories}
            onCategorySelect={handleCategorySelect}
            onMemorySelect={handleMemorySelect}
          />
        )}
        {activeTab === 'timeline' && (
          <MemoryTimeline />
        )}
        {activeTab === 'graph' && (
          <KnowledgeGraph onNodeClick={(nodeId) => console.log('Node clicked:', nodeId)} />
        )}
        {activeTab === 'ai-chat' && (
          <AIChatPanel />
        )}
        {activeTab === 'insights' && (
          <MemoryInsights />
        )}
      </div>
    </div>
  )
}

export default MemoryView
