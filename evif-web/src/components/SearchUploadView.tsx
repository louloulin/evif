/**
 * Phase 9.4: 搜索与上传视图
 * 搜索对接 POST /api/v1/grep，上传对接 /api/v1/fs/write 与 /api/v1/files
 */

import React, { useState, useCallback } from 'react'
import { SearchBar } from '@/components/search/SearchBar'
import { SearchResults } from '@/components/search/SearchResults'
import { UploadDropzone } from '@/components/upload/UploadDropzone'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type { SearchQuery, SearchResponse } from '@/types/search'
import type { UploadFile } from '@/types/upload'
import { searchGrep } from '@/services/search-api'
import { uploadFiles } from '@/services/upload-api'

export const SearchUploadView: React.FC = () => {
  const [searchData, setSearchData] = useState<SearchResponse | null>(null)
  const [searchLoading, setSearchLoading] = useState(false)
  const [searchError, setSearchError] = useState<string | null>(null)
  const [uploads, setUploads] = useState<UploadFile[]>([])
  const [uploadPath, setUploadPath] = useState('/mem')

  const handleSearch = useCallback(async (query: SearchQuery) => {
    setSearchError(null)
    setSearchLoading(true)
    try {
      const res = await searchGrep(query.path, query.query, true)
      setSearchData(res)
    } catch (e) {
      setSearchError(e instanceof Error ? e.message : '搜索失败')
      setSearchData(null)
    } finally {
      setSearchLoading(false)
    }
  }, [])

  const handleClearSearch = useCallback(() => {
    setSearchData(null)
    setSearchError(null)
  }, [])

  const handleUpload = useCallback(
    async (files: File[], path: string) => {
      const basePath = path || uploadPath
      const base = basePath.replace(/\/?$/, '/')
      const fileIds = new Map<string, string>()

      // 初始化上传任务
      for (const f of files) {
        const id = `${Date.now()}-${f.name}`
        fileIds.set(f.name, id)
        setUploads((prev) =>
          prev.concat({
            id,
            file: f,
            path: base + f.name,
            status: 'uploading',
            progress: 0,
            speed: 0,
            uploaded: 0,
            total: f.size,
          })
        )
      }

      // 开始上传(带进度)
      const result = await uploadFiles(files, basePath, (fileName, progress) => {
        const id = fileIds.get(fileName)
        if (id) {
          setUploads((prev) =>
            prev.map((u) =>
              u.id === id
                ? {
                    ...u,
                    progress,
                    uploaded: Math.round((u.total * progress) / 100),
                  }
                : u
            )
          )
        }
      })

      // 更新最终状态
      setUploads((prev) => {
        const next = [...prev]
        const start = next.length - files.length
        result.results.forEach((r, idx) => {
          const i = start + idx
          if (i < next.length) {
            next[i] = {
              ...next[i],
              status: r.success ? 'completed' : 'error',
              progress: 100,
              uploaded: next[i].total,
              error: r.error,
            }
          }
        })
        return next
      })

      if (result.failed > 0) {
        console.error('Upload errors:', result.results.filter((r) => !r.success).map((r) => r.error))
      }
    },
    [uploadPath]
  )

  return (
    <div className="p-4 h-full overflow-auto">
      <Tabs defaultValue="search" className="space-y-4">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="search">搜索 (grep)</TabsTrigger>
          <TabsTrigger value="upload">上传</TabsTrigger>
        </TabsList>
        <TabsContent value="search" className="space-y-4">
          <SearchBar
            onSearch={handleSearch}
            onClear={handleClearSearch}
            loading={searchLoading}
            defaultPath="/mem"
            placeholder="输入正则或关键词，在路径下搜索内容..."
          />
          <SearchResults
            data={searchData}
            loading={searchLoading}
            error={searchError ?? undefined}
          />
        </TabsContent>
        <TabsContent value="upload" className="space-y-4">
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">上传到目录:</span>
            <input
              type="text"
              value={uploadPath}
              onChange={(e) => setUploadPath(e.target.value)}
              className="flex-1 max-w-xs px-3 py-1.5 rounded border bg-background text-sm"
              placeholder="/mem"
            />
          </div>
          <UploadDropzone
            onUpload={handleUpload}
            uploads={uploads}
            defaultPath={uploadPath}
          />
        </TabsContent>
      </Tabs>
    </div>
  )
}

export default SearchUploadView
