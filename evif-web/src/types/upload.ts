// 文件上传/下载类型定义

export type UploadStatus = 'pending' | 'uploading' | 'completed' | 'error' | 'paused'
export type DownloadStatus = 'pending' | 'downloading' | 'completed' | 'error' | 'paused'

export interface UploadFile {
  id: string
  file: File
  path: string
  status: UploadStatus
  progress: number // 0-100
  speed: number // bytes/s
  uploaded: number // bytes
  total: number // bytes
  error?: string
  startTime?: Date
  endTime?: Date
}

export interface DownloadTask {
  id: string
  path: string
  name: string
  status: DownloadStatus
  progress: number // 0-100
  speed: number // bytes/s
  downloaded: number // bytes
  total: number // bytes
  error?: string
  startTime?: Date
  endTime?: Date
}

export interface UploadOptions {
  overwrite?: boolean
  chunkSize?: number // bytes, default 5MB
  maxRetries?: number
  resume?: boolean // 支持断点续传
}

export interface DownloadOptions {
  chunkSize?: number // bytes
  maxRetries?: number
  resume?: boolean
  compress?: boolean // gzip 压缩
}
