// 文件上传/下载类型定义

export type UploadStatus = 'pending' | 'uploading' | 'completed' | 'error' | 'paused'
export type DownloadStatus = 'pending' | 'downloading' | 'completed' | 'error' | 'paused'

export interface UploadFile {
  id: string
  file: File
  path: string
  status: UploadStatus
  progress: number
  speed: number
  uploaded: number
  total: number
  error?: string
  startTime?: Date
  endTime?: Date
}

export interface DownloadTask {
  id: string
  path: string
  name: string
  status: DownloadStatus
  progress: number
  speed: number
  downloaded: number
  total: number
  error?: string
  startTime?: Date
  endTime?: Date
}

export interface UploadOptions {
  overwrite?: boolean
  chunkSize?: number
  maxRetries?: number
  resume?: boolean
}

export interface DownloadOptions {
  chunkSize?: number
  maxRetries?: number
  resume?: boolean
  compress?: boolean
}
