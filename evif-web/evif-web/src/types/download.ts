// 文件下载类型定义

export type DownloadStatus = 'pending' | 'downloading' | 'completed' | 'error' | 'paused'

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
