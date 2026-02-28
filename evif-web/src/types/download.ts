// 文件下载类型定义

export type DownloadStatus = 'pending' | 'downloading' | 'completed' | 'error' | 'paused'

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
