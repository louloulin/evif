import React, { useCallback, useState } from 'react'
import { Upload as UploadIcon, X, File, FolderUp, AlertCircle } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Badge } from '@/components/ui/badge'
import { UploadFile, UploadStatus } from '@/types/upload'

interface UploadDropzoneProps {
  onUpload: (files: File[], path: string) => void
  uploads?: UploadFile[]
  onPause?: (id: string) => void
  onResume?: (id: string) => void
  onCancel?: (id: string) => void
  onRemove?: (id: string) => void
  defaultPath?: string
}

export const UploadDropzone: React.FC<UploadDropzoneProps> = ({
  onUpload,
  uploads = [],
  onPause,
  onResume,
  onCancel,
  onRemove,
  defaultPath = '/',
}) => {
  const [isDragging, setIsDragging] = useState(false)
  const [uploadPath, setUploadPath] = useState(defaultPath)

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
  }, [])

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault()
      setIsDragging(false)

      const files = Array.from(e.dataTransfer.files)
      if (files.length > 0) {
        onUpload(files, uploadPath)
      }
    },
    [onUpload, uploadPath]
  )

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = Array.from(e.target.files || [])
      if (files.length > 0) {
        onUpload(files, uploadPath)
      }
      // 重置 input 以允许重复选择相同文件
      e.target.value = ''
    },
    [onUpload, uploadPath]
  )

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

  const getStatusBadge = (status: UploadStatus) => {
    switch (status) {
      case 'pending':
        return <Badge variant="secondary">等待中</Badge>
      case 'uploading':
        return <Badge variant="default">上传中</Badge>
      case 'completed':
        return <Badge variant="outline" className="bg-green-50 text-green-700">完成</Badge>
      case 'error':
        return <Badge variant="destructive">错误</Badge>
      case 'paused':
        return <Badge variant="secondary">暂停</Badge>
    }
  }

  return (
    <div className="space-y-4">
      {/* 上传区域 */}
      <Card
        className={`border-2 border-dashed transition-colors ${
          isDragging
            ? 'border-primary bg-primary/5'
            : 'border-muted-foreground/25 hover:border-muted-foreground/50'
        }`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <CardContent className="p-8">
          <div className="flex flex-col items-center justify-center text-center space-y-4">
            <div className="p-4 bg-primary/10 rounded-full">
              <FolderUp className="h-8 w-8 text-primary" />
            </div>
            <div>
              <h3 className="font-semibold text-lg mb-1">
                拖拽文件到此处上传
              </h3>
              <p className="text-sm text-muted-foreground">
                或点击下方按钮选择文件
              </p>
            </div>

            {/* 上传路径 */}
            <div className="w-full max-w-md">
              <label className="text-sm font-medium mb-1 block">
                上传到路径
              </label>
              <input
                type="text"
                value={uploadPath}
                onChange={(e) => setUploadPath(e.target.value)}
                className="w-full px-3 py-2 border rounded-md text-sm"
                placeholder="/"
              />
            </div>

            <div className="flex gap-3">
              <Button asChild>
                <label htmlFor="file-upload" className="cursor-pointer">
                  <UploadIcon className="h-4 w-4 mr-2" />
                  选择文件
                </label>
              </Button>
              <input
                id="file-upload"
                type="file"
                multiple
                className="hidden"
                onChange={handleFileSelect}
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 上传列表 */}
      {uploads.length > 0 && (
        <Card>
          <CardContent className="p-4">
            <div className="space-y-3">
              {uploads.map((upload) => (
                <div
                  key={upload.id}
                  className="flex items-center gap-3 p-3 border rounded-lg bg-muted/20"
                >
                  {/* 文件图标 */}
                  <File className="h-5 w-5 text-muted-foreground shrink-0" />

                  {/* 文件信息 */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-medium text-sm truncate">
                        {upload.file.name}
                      </span>
                      {getStatusBadge(upload.status)}
                    </div>

                    {/* 进度条 */}
                    {upload.status === 'uploading' && (
                      <div className="space-y-1">
                        <Progress value={upload.progress} className="h-2" />
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span>
                            {formatSize(upload.uploaded)} / {formatSize(upload.total)}
                          </span>
                          <span>{formatSpeed(upload.speed)}</span>
                        </div>
                      </div>
                    )}

                    {/* 错误信息 */}
                    {upload.status === 'error' && upload.error && (
                      <div className="flex items-center gap-1 text-xs text-destructive">
                        <AlertCircle className="h-3 w-3" />
                        <span>{upload.error}</span>
                      </div>
                    )}

                    {/* 完成信息 */}
                    {upload.status === 'completed' && (
                      <div className="text-xs text-muted-foreground">
                        上传完成 - {formatSize(upload.total)}
                      </div>
                    )}
                  </div>

                  {/* 操作按钮 */}
                  <div className="flex gap-1 shrink-0">
                    {upload.status === 'uploading' && onPause && (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => onPause(upload.id)}
                      >
                        暂停
                      </Button>
                    )}
                    {upload.status === 'paused' && onResume && (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => onResume(upload.id)}
                      >
                        继续
                      </Button>
                    )}
                    {(upload.status === 'pending' ||
                      upload.status === 'error' ||
                      upload.status === 'completed') &&
                      onRemove && (
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => onRemove(upload.id)}
                        >
                          <X className="h-4 w-4" />
                        </Button>
                      )}
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
