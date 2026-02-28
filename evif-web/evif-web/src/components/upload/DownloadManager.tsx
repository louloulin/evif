import React, { useState } from 'react'
import { Download as DownloadIcon, Folder, Trash2, Pause, Play, X, FileDown } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { DownloadTask, DownloadStatus } from '@/types/download'

interface DownloadManagerProps {
  downloads?: DownloadTask[]
  onPause?: (id: string) => void
  onResume?: (id: string) => void
  onCancel?: (id: string) => void
  onRemove?: (id: string) => void
  onClearCompleted?: () => void
}

export const DownloadManager: React.FC<DownloadManagerProps> = ({
  downloads = [],
  onPause,
  onResume,
  onCancel,
  onRemove,
  onClearCompleted,
}) => {
  const [filter, setFilter] = useState<'all' | 'downloading' | 'completed'>('all')

  const formatSpeed = (bytesPerSecond: number): string => {
    const mb = bytesPerSecond / (1024 * 1024)
    if (mb >= 1) return `${mb.toFixed(2)} MB/s`
    return `${(bytesPerSecond / 1024).toFixed(2)} KB/s`
  }

  const formatSize = (bytes: number): string => {
    const mb = bytes / (1024 * 1024)
    if (mb >= 1) return `${mb.toFixed(2)} MB`
    return `${(bytes / 1024).toFixed(2)} KB`
  }

  const getStatusBadge = (status: DownloadStatus) => {
    switch (status) {
      case 'pending':
        return <Badge variant="secondary">等待中</Badge>
      case 'downloading':
        return <Badge variant="default">下载中</Badge>
      case 'completed':
        return <Badge variant="outline" className="bg-green-50 text-green-700">完成</Badge>
      case 'error':
        return <Badge variant="destructive">错误</Badge>
      case 'paused':
        return <Badge variant="secondary">暂停</Badge>
    }
  }

  const filteredDownloads = downloads.filter((download) => {
    if (filter === 'all') return true
    return download.status === filter
  })

  const completedCount = downloads.filter((d) => d.status === 'completed').length
  const downloadingCount = downloads.filter((d) => d.status === 'downloading').length

  const totalProgress =
    downloads.length > 0
      ? downloads.reduce((sum, d) => sum + d.progress, 0) / downloads.length
      : 0

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <FileDown className="h-5 w-5" />
            下载管理器
          </CardTitle>
          <div className="flex items-center gap-2">
            <Badge variant="outline">{completedCount} 已完成</Badge>
            <Badge variant="outline">{downloadingCount} 下载中</Badge>
          </div>
        </div>

        {/* 过滤器 */}
        <div className="flex gap-2 mt-4">
          <Button
            variant={filter === 'all' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('all')}
          >
            全部 ({downloads.length})
          </Button>
          <Button
            variant={filter === 'downloading' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('downloading')}
          >
            下载中 ({downloadingCount})
          </Button>
          <Button
            variant={filter === 'completed' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('completed')}
          >
            已完成 ({completedCount})
          </Button>
        </div>
      </CardHeader>

      <CardContent>
        {downloads.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <DownloadIcon className="h-12 w-12 mx-auto mb-3 opacity-50" />
            <p>暂无下载任务</p>
          </div>
        ) : (
          <>
            {/* 总体进度 */}
            {downloadingCount > 0 && (
              <div className="mb-4 p-3 bg-muted/20 rounded-lg">
                <div className="flex items-center justify-between text-sm mb-2">
                  <span className="font-medium">总体进度</span>
                  <span className="text-muted-foreground">
                    {Math.round(totalProgress)}%
                  </span>
                </div>
                <Progress value={totalProgress} className="h-2" />
              </div>
            )}

            {/* 下载列表 */}
            <ScrollArea className="h-[400px]">
              <div className="space-y-2 pr-4">
                {filteredDownloads.map((download) => (
                  <div
                    key={download.id}
                    className="p-3 border rounded-lg bg-card hover:bg-accent/50 transition-colors"
                  >
                    <div className="flex items-start gap-3">
                      {/* 文件图标 */}
                      <Folder className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />

                      {/* 文件信息 */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-2">
                          <span className="font-medium text-sm truncate">
                            {download.name}
                          </span>
                          {getStatusBadge(download.status)}
                        </div>

                        {/* 路径 */}
                        <div className="text-xs text-muted-foreground mb-2 truncate">
                          {download.path}
                        </div>

                        {/* 进度条 */}
                        {download.status === 'downloading' && (
                          <div className="space-y-1">
                            <Progress value={download.progress} className="h-2" />
                            <div className="flex items-center justify-between text-xs text-muted-foreground">
                              <span>
                                {formatSize(download.downloaded)} /{' '}
                                {formatSize(download.total)}
                              </span>
                              <span>{formatSpeed(download.speed)}</span>
                            </div>
                          </div>
                        )}

                        {/* 错误信息 */}
                        {download.status === 'error' && download.error && (
                          <div className="text-xs text-destructive mt-1">
                            {download.error}
                          </div>
                        )}

                        {/* 完成信息 */}
                        {download.status === 'completed' && (
                          <div className="text-xs text-muted-foreground">
                            已完成 - {formatSize(download.total)}
                          </div>
                        )}

                        {/* 暂停信息 */}
                        {download.status === 'paused' && (
                          <div className="text-xs text-muted-foreground">
                            已暂停 - {formatSize(download.downloaded)} /{' '}
                            {formatSize(download.total)}
                          </div>
                        )}
                      </div>

                      {/* 操作按钮 */}
                      <div className="flex gap-1 shrink-0">
                        {download.status === 'downloading' && onPause && (
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => onPause(download.id)}
                            title="暂停"
                          >
                            <Pause className="h-4 w-4" />
                          </Button>
                        )}
                        {download.status === 'paused' && onResume && (
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => onResume(download.id)}
                            title="继续"
                          >
                            <Play className="h-4 w-4" />
                          </Button>
                        )}
                        {(download.status === 'pending' ||
                          download.status === 'paused' ||
                          download.status === 'error') &&
                          onCancel && (
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => onCancel(download.id)}
                              title="取消"
                            >
                              <X className="h-4 w-4" />
                            </Button>
                          )}
                        {download.status === 'completed' && onRemove && (
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => onRemove(download.id)}
                            title="删除"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </ScrollArea>

            {/* 清除已完成 */}
            {completedCount > 0 && onClearCompleted && (
              <div className="mt-4 pt-4 border-t">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onClearCompleted}
                  className="w-full"
                >
                  清除已完成的下载 ({completedCount})
                </Button>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  )
}
