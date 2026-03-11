import React, { useState, useMemo } from 'react'
import { File, FileCode, FileText, Image, Archive, Music, Video, FolderOpen, Search } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Pagination } from '@/components/ui/pagination'
import { SearchResult, SearchResponse } from '@/types/search'

// 分页配置:每页50个文件
const PAGE_SIZE = 50

interface SearchResultsProps {
  data: SearchResponse | null
  loading?: boolean
  error?: string
  onResultClick?: (result: SearchResult) => void
}

export const SearchResults: React.FC<SearchResultsProps> = ({
  data,
  loading = false,
  error,
  onResultClick,
}) => {
  const [selectedResult, setSelectedResult] = useState<string | null>(null)
  const [currentPage, setCurrentPage] = useState(1)

  // 获取文件图标
  const getFileIcon = (path: string, isDirectory?: boolean) => {
    if (isDirectory) {
      return <FolderOpen className="h-4 w-4 text-blue-500" />
    }

    const ext = path.split('.').pop()?.toLowerCase()
    switch (ext) {
      case 'ts':
      case 'tsx':
      case 'js':
      case 'jsx':
      case 'json':
        return <FileCode className="h-4 w-4 text-blue-500" />
      case 'txt':
      case 'md':
        return <FileText className="h-4 w-4 text-gray-500" />
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'gif':
      case 'svg':
        return <Image className="h-4 w-4 text-purple-500" />
      case 'mp3':
      case 'wav':
        return <Music className="h-4 w-4 text-green-500" />
      case 'mp4':
      case 'avi':
        return <Video className="h-4 w-4 text-red-500" />
      case 'zip':
      case 'tar':
      case 'gz':
        return <Archive className="h-4 w-4 text-orange-500" />
      default:
        return <File className="h-4 w-4 text-gray-500" />
    }
  }

  // 高亮匹配文本
  const highlightMatch = (text: string, pattern: string) => {
    if (!pattern) return text

    try {
      const regex = new RegExp(`(${pattern})`, 'gi')
      const parts = text.split(regex)
      return parts.map((part, index) =>
        regex.test(part) ? (
          <mark key={index} className="bg-yellow-200 dark:bg-yellow-800 rounded px-0.5">
            {part}
          </mark>
        ) : (
          part
        )
      )
    } catch {
      return text
    }
  }

  // 格式化文件大小
  const formatSize = (bytes?: number): string => {
    if (!bytes) return '-'
    const units = ['B', 'KB', 'MB', 'GB']
    let size = bytes
    let unitIndex = 0
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }
    return `${size.toFixed(1)} ${units[unitIndex]}`
  }

  // 按文件分组结果
  const groupedResults = useMemo(() => {
    if (!data?.results) return {}

    return data.results.reduce((acc, result) => {
      const filePath = result.path
      if (!acc[filePath]) {
        acc[filePath] = []
      }
      acc[filePath].push(result)
      return acc
    }, {} as Record<string, SearchResult[]>)
  }, [data])

  // 分页逻辑
  const allFiles = Object.keys(groupedResults)
  const totalPages = Math.ceil(allFiles.length / PAGE_SIZE)
  const needPagination = allFiles.length > PAGE_SIZE

  // 当前页的文件列表
  const currentPageFiles = useMemo(() => {
    if (!needPagination) {
      return allFiles
    }
    const start = (currentPage - 1) * PAGE_SIZE
    const end = start + PAGE_SIZE
    return allFiles.slice(start, end)
  }, [allFiles, currentPage, needPagination])

  // 当搜索结果变化时重置到第一页
  useMemo(() => {
    setCurrentPage(1)
  }, [data?.results?.length])

  if (loading) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-muted-foreground">
            搜索中...
          </div>
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-destructive">
            {error}
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!data || data.results.length === 0) {
    return (
      <Card>
        <CardContent className="p-12">
          <div className="flex flex-col items-center justify-center text-center space-y-4">
            <Search className="h-16 w-16 text-muted-foreground/50" />
            <div>
              <h3 className="text-lg font-semibold text-foreground">
                未找到结果
              </h3>
              <p className="text-sm text-muted-foreground mt-1">
                尝试调整搜索关键词或搜索路径
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-4">
      {/* 搜索结果统计 */}
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-4">
          <span className="text-muted-foreground">
            找到 <span className="font-semibold text-foreground">{data.matches}</span> 个匹配
          </span>
          <span className="text-muted-foreground">
            在 <span className="font-semibold text-foreground">{allFiles.length}</span> 个文件中
          </span>
          {needPagination && (
            <span className="text-muted-foreground">
              第 <span className="font-semibold text-foreground">{currentPage}</span> / {totalPages} 页
            </span>
          )}
        </div>
      </div>

      {/* 搜索结果列表 */}
      <ScrollArea className="h-[600px]">
        <div className="space-y-2 pr-4">
          {currentPageFiles.map((filePath) => {
            const results = groupedResults[filePath]
            return (
              <Card
                key={filePath}
                className={`cursor-pointer transition-colors hover:bg-accent ${
                  selectedResult === filePath ? 'bg-accent' : ''
                }`}
                onClick={() => {
                  setSelectedResult(filePath)
                  onResultClick?.(results[0])
                }}
              >
              <CardContent className="p-4">
                <div className="flex items-start gap-4">
                  {/* 文件图标 */}
                  <div className="mt-0.5">{getFileIcon(filePath)}</div>

                  <div className="flex-1 min-w-0">
                    {/* 文件路径 */}
                    <div className="flex items-center gap-2 mb-2">
                      <span className="font-medium text-sm truncate">
                        {filePath}
                      </span>
                      {results[0].size && (
                        <Badge variant="secondary" className="text-xs">
                          {formatSize(results[0].size)}
                        </Badge>
                      )}
                    </div>

                    {/* 匹配项列表 */}
                    <div className="space-y-1">
                      {results.slice(0, 5).map((result, index) => (
                        <div
                          key={index}
                          className="text-xs font-mono bg-muted/30 rounded px-2 py-1 truncate"
                        >
                          {result.line_number && (
                            <span className="text-muted-foreground mr-2">
                              Line {result.line_number}:
                            </span>
                          )}
                          {result.line && highlightMatch(result.line, data.pattern)}
                        </div>
                      ))}
                      {results.length > 5 && (
                        <div className="text-xs text-muted-foreground">
                          还有 {results.length - 5} 个匹配...
                        </div>
                      )}
                    </div>
                  </div>

                  {/* 匹配数量 */}
                  <Badge variant="outline" className="shrink-0">
                    {results.length}
                  </Badge>
                </div>
              </CardContent>
            </Card>
            )
          })}
        </div>
      </ScrollArea>

      {/* 分页控件 */}
      {needPagination && (
        <div className="flex justify-center">
          <Pagination
            currentPage={currentPage}
            totalPages={totalPages}
            onPageChange={(page) => {
              setCurrentPage(page)
              // 滚动到顶部
              window.scrollTo({ top: 0, behavior: 'smooth' })
            }}
          />
        </div>
      )}
    </div>
  )
}
