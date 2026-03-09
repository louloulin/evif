/**
 * MemoryView - 记忆管理视图
 *
 * 主容器组件，包含 MemoryExplorer、CategoryView 和 MemoryTimeline
 */

import React, { useState } from 'react'
import MemoryExplorer from './MemoryExplorer'
import CategoryView from './CategoryView'
import MemoryTimeline from './MemoryTimeline'
import KnowledgeGraph from './KnowledgeGraph'
import type { MemoryItem, Category } from '@/services/memory-api'

type MemoryViewTab = 'explorer' | 'timeline' | 'graph'

const MemoryView: React.FC = () => {
  const [activeTab, setActiveTab] = useState<MemoryViewTab>('explorer')
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null)
  const [selectedMemory, setSelectedMemory] = useState<MemoryItem | null>(null)

  // 处理分类选择
  const handleCategorySelect = (category: Category) => {
    setSelectedCategoryId(category.id)
  }

  // 处理记忆选择
  const handleMemorySelect = (memory: MemoryItem) => {
    setSelectedMemory(memory)
  }

  // 返回分类列表
  const handleBackToCategories = () => {
    setSelectedCategoryId(null)
  }

  // 如果选择了分类，显示分类详情
  if (selectedCategoryId) {
    return (
      <CategoryView
        categoryId={selectedCategoryId}
        onBack={handleBackToCategories}
        onMemorySelect={handleMemorySelect}
      />
    )
  }

  return (
    <div className="memory-view">
      {/* Tab 导航 */}
      <div className="memory-view-tabs">
        <button
          className={`tab-button ${activeTab === 'explorer' ? 'active' : ''}`}
          onClick={() => setActiveTab('explorer')}
        >
          记忆浏览器
        </button>
        <button
          className={`tab-button ${activeTab === 'timeline' ? 'active' : ''}`}
          onClick={() => setActiveTab('timeline')}
        >
          时间线
        </button>
        <button
          className={`tab-button ${activeTab === 'graph' ? 'active' : ''}`}
          onClick={() => setActiveTab('graph')}
        >
          知识图谱
        </button>
      </div>

      {/* Tab 内容 */}
      <div className="memory-view-content">
        {activeTab === 'explorer' && (
          <MemoryExplorer
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
      </div>

      {/* 选中的记忆详情面板 */}
      {selectedMemory && (
        <div className="memory-detail-panel">
          <div className="detail-header">
            <h4>记忆详情</h4>
            <button
              className="close-btn"
              onClick={() => setSelectedMemory(null)}
            >
              ×
            </button>
          </div>
          <div className="detail-content">
            <div className="detail-field">
              <label>ID:</label>
              <span>{selectedMemory.id}</span>
            </div>
            <div className="detail-field">
              <label>类型:</label>
              <span>{selectedMemory.type}</span>
            </div>
            <div className="detail-field">
              <label>摘要:</label>
              <span>{selectedMemory.summary}</span>
            </div>
            <div className="detail-field">
              <label>创建时间:</label>
              <span>{selectedMemory.created}</span>
            </div>
            <div className="detail-field">
              <label>更新时间:</label>
              <span>{selectedMemory.updated}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default MemoryView
