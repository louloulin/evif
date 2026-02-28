import React, { useState } from 'react'
import { File, Filter, SlidersHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { SearchOptions, FileTypeFilter } from '@/types/search'

interface FilterPanelProps {
  options: SearchOptions
  onChange: (options: SearchOptions) => void
  onReset: () => void
  resultCount?: number
}

export const FilterPanel: React.FC<FilterPanelProps> = ({
  options,
  onChange,
  onReset,
  resultCount,
}) => {
  const [fileType, setFileType] = useState<FileTypeFilter>('all')
  const [fileExtensions, setFileExtensions] = useState<string[]>([])
  const [minSize, setMinSize] = useState<string>('')
  const [maxSize, setMaxSize] = useState<string>('')

  const handleAddExtension = () => {
    // 实现添加扩展名逻辑
  }

  const handleRemoveExtension = (ext: string) => {
    const updated = fileExtensions.filter(e => e !== ext)
    setFileExtensions(updated)
    onChange({ ...options, fileTypes: updated })
  }

  const handleReset = () => {
    setFileType('all')
    setFileExtensions([])
    setMinSize('')
    setMaxSize('')
    onReset()
  }

  const formatSize = (sizeStr: string): number | undefined => {
    if (!sizeStr) return undefined
    const num = parseFloat(sizeStr)
    if (isNaN(num)) return undefined

    // 支持 KB, MB, GB 后缀
    const unit = sizeStr.toLowerCase().match(/[kmg]b?$/)?.[0]
    switch (unit) {
      case 'kb': return num * 1024
      case 'mb': return num * 1024 * 1024
      case 'gb': return num * 1024 * 1024 * 1024
      default: return num
    }
  }

  const hasActiveFilters =
    fileType !== 'all' ||
    fileExtensions.length > 0 ||
    minSize ||
    maxSize

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <SlidersHorizontal className="h-4 w-4" />
            过滤选项
          </CardTitle>
          {hasActiveFilters && (
            <Button variant="ghost" size="sm" onClick={handleReset}>
              重置
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 文件类型过滤 */}
        <div className="space-y-2">
          <Label>文件类型</Label>
          <div className="flex flex-wrap gap-2">
            <Badge
              variant={fileType === 'all' ? 'default' : 'outline'}
              className="cursor-pointer"
              onClick={() => {
                setFileType('all')
                onChange({ ...options, fileTypes: undefined })
              }}
            >
              全部
            </Badge>
            <Badge
              variant={fileType === 'file' ? 'default' : 'outline'}
              className="cursor-pointer"
              onClick={() => setFileType('file')}
            >
              <File className="h-3 w-3 mr-1" />
              文件
            </Badge>
            <Badge
              variant={fileType === 'directory' ? 'default' : 'outline'}
              className="cursor-pointer"
              onClick={() => setFileType('directory')}
            >
              目录
            </Badge>
          </div>
        </div>

        {/* 文件扩展名过滤 */}
        <div className="space-y-2">
          <Label>文件扩展名</Label>
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder="输入扩展名 (如 ts,tsx)"
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  const input = e.currentTarget
                  const ext = input.value.trim().replace(/^\./, '')
                  if (ext && !fileExtensions.includes(ext)) {
                    const updated = [...fileExtensions, ext]
                    setFileExtensions(updated)
                    onChange({ ...options, fileTypes: updated })
                    input.value = ''
                  }
                }
              }}
              className="h-9"
            />
            <Button size="sm" variant="outline" onClick={handleAddExtension}>
              添加
            </Button>
          </div>
          {fileExtensions.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-2">
              {fileExtensions.map((ext) => (
                <Badge key={ext} variant="secondary" className="gap-2">
                  .{ext}
                  <button
                    onClick={() => handleRemoveExtension(ext)}
                    className="ml-1 hover:text-destructive"
                  >
                    ×
                  </button>
                </Badge>
              ))}
            </div>
          )}
        </div>

        {/* 文件大小过滤 */}
        <div className="space-y-2">
          <Label>文件大小</Label>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <Input
                type="text"
                placeholder="最小 (如 1KB)"
                value={minSize}
                onChange={(e) => {
                  setMinSize(e.target.value)
                  onChange({
                    ...options,
                    minSize: formatSize(e.target.value),
                  })
                }}
                className="h-9"
              />
            </div>
            <div>
              <Input
                type="text"
                placeholder="最大 (如 10MB)"
                value={maxSize}
                onChange={(e) => {
                  setMaxSize(e.target.value)
                  onChange({
                    ...options,
                    maxSize: formatSize(e.target.value),
                  })
                }}
                className="h-9"
              />
            </div>
          </div>
        </div>

        {/* 结果统计 */}
        {resultCount !== undefined && (
          <div className="pt-2 border-t text-sm text-muted-foreground">
            找到 {resultCount} 个结果
          </div>
        )}
      </CardContent>
    </Card>
  )
}
