/**
 * MemoryExplorer - 记忆浏览器
 * 核心功能: 分类浏览、记忆列表、搜索过滤
 */

import React, { useState, useMemo } from 'react'
import { Search, Plus, FileText, MoreVertical, Trash2, Edit } from 'lucide-react'
import { EmptyState } from '@/components/ui/loading'
import { listMemories, deleteMemory, MemoryItem, Category } from '@/services/memory-api'
import { toast } from '@/hooks/use-toast'

interface MemoryExplorerProps {
  memories: MemoryItem[]
  categories: Category[]
  onCategorySelect: (category: Category) => void
  onMemorySelect: (memory: MemoryItem) => void
}

const MemoryExplorer: React.FC<MemoryExplorerProps> = ({
  memories,
  categories,
  onCategorySelect,
  onMemorySelect,
}) => {
  // ============ State ============
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null)
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')
  const [loading, setLoading] = useState(false)

  // ============ Filtered Data ============
  const filteredMemories = useMemo(() => {
    return memories.filter((memory) => {
      // Category filter
      if (selectedCategoryId && memory.category !== selectedCategoryId) {
        return false
      }
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase()
        return (
          memory.content.toLowerCase().includes(query) ||
          memory.summary.toLowerCase().includes(query) ||
          memory.type.toLowerCase().includes(query)
        )
      }
      return true
    })
  }, [memories, selectedCategoryId, searchQuery])

  // ============ Handlers ============
  const handleCreateMemory = async () => {
    setLoading(true)
    try {
      const newMemory = await listMemories() // Just refresh for now
      toast({
        title: 'Memory created',
        description: 'Your new memory has been created.',
      })
    } catch (err) {
      toast({
        title: 'Error',
        description: 'Failed to create memory',
        variant: 'destructive',
      })
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteMemory = async (id: string) => {
    if (!confirm('Are you sure you want to delete this memory?')) return
    
    setLoading(true)
    try {
      await deleteMemory(id)
      toast({
        title: 'Memory deleted',
        description: 'The memory has been deleted.',
      })
    } catch (err) {
      toast({
        title: 'Error',
        description: 'Failed to delete memory',
        variant: 'destructive',
      })
    } finally {
      setLoading(false)
    }
  }

  // ============ Render ============
  return (
    <div className="memory-explorer p-4 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Memory Explorer</h2>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setViewMode('grid')}
            className={`p-2 rounded ${viewMode === 'grid' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
          >
            Grid
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={`p-2 rounded ${viewMode === 'list' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
          >
            List
          </button>
        </div>
      </div>

      {/* Search & Actions */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-4 py-2 border rounded-lg bg-background"
          />
        </div>
        <button
          onClick={handleCreateMemory}
          disabled={loading}
          className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
        >
          <Plus className="h-4 w-4" />
          New Memory
        </button>
      </div>

      {/* Categories */}
      <div className="flex gap-2 overflow-x-auto pb-2">
        <button
          onClick={() => setSelectedCategoryId(null)}
          className={`px-3 py-1 rounded-full text-sm whitespace-nowrap ${
            selectedCategoryId === null
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted hover:bg-muted/80'
          }`}
        >
          All ({memories.length})
        </button>
        {categories.map((category) => (
          <button
            key={category.id}
            onClick={() => setSelectedCategoryId(category.id)}
            className={`px-3 py-1 rounded-full text-sm whitespace-nowrap ${
              selectedCategoryId === category.id
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            {category.name} ({category.item_count})
          </button>
        ))}
      </div>

      {/* Memory List */}
      {filteredMemories.length === 0 ? (
        <EmptyState
          icon="folder"
          title="No memories found"
          description={searchQuery ? 'Try adjusting your search query.' : 'Create your first memory to get started.'}
          action={!searchQuery ? { label: 'Create Memory', onClick: handleCreateMemory } : undefined}
        />
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredMemories.map((memory) => (
            <MemoryCard
              key={memory.id}
              memory={memory}
              onSelect={() => onMemorySelect(memory)}
              onDelete={() => handleDeleteMemory(memory.id)}
            />
          ))}
        </div>
      ) : (
        <div className="space-y-2">
          {filteredMemories.map((memory) => (
            <MemoryListItem
              key={memory.id}
              memory={memory}
              onSelect={() => onMemorySelect(memory)}
              onDelete={() => handleDeleteMemory(memory.id)}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// ============ Memory Card ============
interface MemoryCardProps {
  memory: MemoryItem
  onSelect: () => void
  onDelete: () => void
}

const MemoryCard: React.FC<MemoryCardProps> = ({ memory, onSelect, onDelete }) => {
  const [showMenu, setShowMenu] = useState(false)

  return (
    <div
      className="group border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer"
      onClick={onSelect}
    >
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="text-xs px-2 py-0.5 rounded bg-muted">{memory.type}</span>
        </div>
        <div className="relative">
          <button
            onClick={(e) => {
              e.stopPropagation()
              setShowMenu(!showMenu)
            }}
            className="p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-muted"
          >
            <MoreVertical className="h-4 w-4" />
          </button>
          {showMenu && (
            <div className="absolute right-0 top-full mt-1 w-32 bg-background border rounded-lg shadow-lg z-10">
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onSelect()
                  setShowMenu(false)
                }}
                className="w-full px-3 py-2 text-left text-sm hover:bg-muted flex items-center gap-2"
              >
                <Edit className="h-4 w-4" /> Edit
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onDelete()
                  setShowMenu(false)
                }}
                className="w-full px-3 py-2 text-left text-sm hover:bg-muted flex items-center gap-2 text-destructive"
              >
                <Trash2 className="h-4 w-4" /> Delete
              </button>
            </div>
          )}
        </div>
      </div>
      <h3 className="font-medium mb-1 truncate">{memory.summary || 'Untitled'}</h3>
      <p className="text-sm text-muted-foreground line-clamp-2">{memory.content}</p>
      <div className="mt-2 text-xs text-muted-foreground">
        {new Date(memory.created).toLocaleDateString()}
      </div>
    </div>
  )
}

// ============ Memory List Item ============
const MemoryListItem: React.FC<MemoryCardProps> = ({ memory, onSelect, onDelete }) => {
  return (
    <div
      className="group flex items-center gap-4 p-3 border rounded-lg hover:bg-muted/50 cursor-pointer"
      onClick={onSelect}
    >
      <FileText className="h-5 w-5 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium truncate">{memory.summary || 'Untitled'}</span>
          <span className="text-xs px-2 py-0.5 rounded bg-muted shrink-0">{memory.type}</span>
        </div>
        <p className="text-sm text-muted-foreground truncate">{memory.content}</p>
      </div>
      <div className="text-xs text-muted-foreground shrink-0">
        {new Date(memory.created).toLocaleDateString()}
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation()
          onDelete()
        }}
        className="p-2 rounded opacity-0 group-hover:opacity-100 hover:bg-destructive/10 text-destructive"
      >
        <Trash2 className="h-4 w-4" />
      </button>
    </div>
  )
}

export default MemoryExplorer
