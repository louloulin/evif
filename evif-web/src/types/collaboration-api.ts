// 协作相关类型定义
import type { Permission, SharePermission, Comment, Activity, Share } from '@/types/collaboration'

export interface CreateShareRequest {
  fileId: string
  filePath: string
  fileName: string
  accessType: 'anyone' | 'specific' | 'link'
  permissions: Permission[]
  expiresAt?: Date
}

export interface ShareResponse {
  id: string
  accessUrl: string
  createdAt: Date
  expiresAt?: Date
}

export interface ListSharesResponse {
  shares: Share[]
  total: number
}

export interface RevokeShareResponse {
  success: boolean
  message: string
}

export interface SetPermissionsRequest {
  filePath: string
  permissions: SharePermission[]
}

export interface SetPermissionsResponse {
  success: boolean
  message: string
}

export interface CreateCommentRequest {
  fileId: string
  filePath: string
  content: string
  lineNumber?: number
  column?: number
  replyTo?: string
}

export interface CommentResponse {
  id: string
  createdAt: Date
}

export interface ListCommentsResponse {
  comments: Comment[]
  total: number
}

export interface UpdateCommentRequest {
  commentId: string
  content: string
}

export interface ResolveCommentResponse {
  success: boolean
  message: string
}

export interface DeleteCommentResponse {
  success: boolean
  message: string
}

export interface GetActivitiesRequest {
  filePath?: string
  type?: 'all' | 'file' | 'share' | 'comment'
  limit?: number
  offset?: number
}

export interface ActivitiesResponse {
  activities: Activity[]
  total: number
}

export interface User {
  id: string
  name: string
  email: string
  avatar?: string
}

export interface UsersResponse {
  users: User[]
  total: number
}

export interface Notification {
  id: string
  type: 'new_comment' | 'file_shared' | 'access_granted' | 'comment_mentioned'
  title: string
  message: string
  fileId?: string
  commentId?: string
  userId: string
  userName: string
  timestamp: Date
  read: boolean
}
