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
    <div className="space-y-4">
      <div className="flex gap-3">
        <div className="relative flex-1">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
          <Input
            type="text"
            placeholder={placeholder}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyPress={handleKeyPress}
            className="pl-10 pr-10 h-10"
            disabled={loading}
          />
          {query && (
            <button
              onClick={handleClear}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              aria-label="清除搜索内容"
            >
              <X className="h-5 w-5" />
            </button>
          )}
        </div>

        <Button
          onClick={handleSearch}
          disabled={!query.trim() || loading}
          size="default"
          className="h-10 px-6"
        >
          {loading ? '搜索中...' : '搜索'}
        </Button>

        <Button
          variant="outline"
          size="icon"
          onClick={() => setShowAdvanced(!showAdvanced)}
          title="高级选项"
          aria-label="展开高级搜索选项"
          className="h-10 w-10"
        >
          <Filter className="h-4 w-4" />
        </Button>
      </div>

      {showAdvanced && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 p-4 border rounded-lg bg-muted/20">
          <div className="space-y-3">
            <label className="text-sm font-medium">搜索类型</label>
            <div className="flex gap-3">
              <Button
                variant={searchType === 'filename' ? 'default' : 'outline'}
                size="default"
                onClick={() => setSearchType('filename')}
                className="flex-1 h-9"
              >
                文件名
              </Button>
              <Button
                variant={searchType === 'content' ? 'default' : 'outline'}
                size="default"
                onClick={() => setSearchType('content')}
                className="flex-1 h-9"
              >
                内容
              </Button>
              <Button
                variant={searchType === 'regex' ? 'default' : 'outline'}
                size="default"
                onClick={() => setSearchType('regex')}
                className="flex-1 h-9"
              >
                正则
              </Button>
            </div>
          </div>

          <div className="space-y-3">
            <label className="text-sm font-medium">搜索路径</label>
            <Input
              type="text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/"
              className="h-10"
            />
          </div>

          <div className="col-span-1 md:col-span-2 flex items-center gap-3">
            <input
              type="checkbox"
              id="case-sensitive"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
              className="rounded w-4 h-4"
            />
            <label htmlFor="case-sensitive" className="text-sm cursor-pointer">
              区分大小写
            </label>
          </div>
        </div>
      )}
    </div>
  )
}
