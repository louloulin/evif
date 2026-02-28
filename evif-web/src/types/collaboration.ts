// 协作功能类型定义

export type Permission = 'read' | 'write' | 'execute' | 'admin'

export interface Share {
  id: string
  fileId: string
  fileName: string
  filePath: string
  createdBy: string
  createdAt: Date
  expiresAt?: Date
  accessUrl: string
  permissions: SharePermission[]
  accessCount: number
}

export interface SharePermission {
  userId: string
  userName: string
  permissions: Permission[]
}

export interface Comment {
  id: string
  fileId: string
  filePath: string
  content: string
  author: string
  authorId: string
  lineNumber?: number
  column?: number
  replyTo?: string
  createdAt: Date
  updatedAt?: Date
  resolved?: boolean
}

export interface Activity {
  id: string
  type: 'create' | 'update' | 'delete' | 'share' | 'comment'
  fileId: string

  filePath: string
  fileName: string
  description: string
  userId: string
  userName: string
  timestamp: Date
  metadata?: Record<string, any>
}
