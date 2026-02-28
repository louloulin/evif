import React, { useState } from 'react'
import { Search as SearchIcon, X, Filter, Clock } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { SearchQuery, SearchType } from '@/types/search'

interface SearchBarProps {
  onSearch: (query: SearchQuery) => void
  onClear: () => void
  loading?: boolean
  placeholder?: string
  defaultPath?: string
}

export const SearchBar: React.FC<SearchBarProps> = ({
  onSearch,
  onClear,
  loading = false,
  placeholder = '搜索文件或内容...',
  defaultPath = '/',
}) => {
  const [query, setQuery] = useState('')
  const [searchType, setSearchType] = useState<SearchType>('content')
  const [path, setPath] = useState(defaultPath)
  const [caseSensitive, setCaseSensitive] = useState(false)
  const [showAdvanced, setShowAdvanced] = useState(false)

  const handleSearch = () => {
    if (!query.trim()) return

    onSearch({
      query: query.trim(),
      type: searchType,
      path,
      caseSensitive,
      maxResults: 100,
    })
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSearch()
    } else if (e.key === 'Escape') {
      handleClear()
    }
  }

  const handleClear = () => {
    setQuery('')
    onClear()
  }

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <div className="relative flex-1">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            type="text"
            placeholder={placeholder}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyPress={handleKeyPress}
            className="pl-9 pr-10"
            disabled={loading}
          />
          {query && (
            <button
              onClick={handleClear}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>

        <Button
          onClick={handleSearch}
          disabled={!query.trim() || loading}
          size="default"
        >
          {loading ? '搜索中...' : '搜索'}
        </Button>

        <Button
          variant="outline"
          size="icon"
          onClick={() => setShowAdvanced(!showAdvanced)}
          title="高级选项"
        >
          <Filter className="h-4 w-4" />
        </Button>
      </div>

      {showAdvanced && (
        <div className="grid grid-cols-2 gap-3 p-3 border rounded-lg bg-muted/20">
          <div className="space-y-2">
            <label className="text-sm font-medium">搜索类型</label>
            <div className="flex gap-2">
              <Button
                variant={searchType === 'filename' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSearchType('filename')}
                className="flex-1"
              >
                文件名
              </Button>
              <Button
                variant={searchType === 'content' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSearchType('content')}
                className="flex-1"
              >
                内容
              </Button>
              <Button
                variant={searchType === 'regex' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSearchType('regex')}
                className="flex-1"
              >
                正则
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">搜索路径</label>
            <Input
              type="text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/"
              className="h-9"
            />
          </div>

          <div className="col-span-2 flex items-center gap-2">
            <input
              type="checkbox"
              id="case-sensitive"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
              className="rounded"
            />
            <label htmlFor="case-sensitive" className="text-sm">
              区分大小写
            </label>
          </div>
        </div>
      )}
    </div>
  )
}
