// 搜索功能类型定义

export type SearchType = 'filename' | 'content' | 'regex'
export type FileTypeFilter = 'all' | 'file' | 'directory'
export type SearchStatus = 'idle' | 'searching' | 'completed' | 'error'

export interface SearchQuery {
  query: string
  type: SearchType
  path: string
  caseSensitive?: boolean
  maxResults?: number
}

export interface SearchOptions {
  fileTypes?: string[]
  minSize?: number
  maxSize?: number
  modifiedAfter?: Date
  modifiedBefore?: Date
}

export interface SearchResult {
  path: string
  line_number?: number
  line?: string
  matches?: number
  preview?: string
  file_type?: string
  size?: number
  modified?: string
}

export interface SearchResponse {
  path: string
  pattern: string
  matches: number
  results: SearchResult[]
}

export interface SearchHistory {
  id: string
  query: SearchQuery
  timestamp: Date
  resultCount: number
  duration: number
}
