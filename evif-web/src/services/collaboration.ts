// 协作 API 服务

import { Permission, SharePermission, Comment, Activity } from '@/types/collaboration'
import type { User } from '@/types/collaboration-api'
import type {
  CreateShareRequest,
  ShareResponse,
  ListSharesResponse,
  RevokeShareResponse,
  SetPermissionsRequest,
  SetPermissionsResponse,
  CreateCommentRequest,
  CommentResponse,
  ListCommentsResponse,
  UpdateCommentRequest,
  ResolveCommentResponse,
  DeleteCommentResponse,
  GetActivitiesRequest,
  ActivitiesResponse,
  UsersResponse,
  Notification,
} from '@/types/collaboration-api'

const API_BASE = import.meta.env?.VITE_API_BASE || '/api/v1'
import { httpFetch } from '@/lib/http'

// 通用错误处理
const handleApiError = (error: unknown): never => {
  if (error instanceof Error) {
    throw new Error(`API Error: ${error.message}`)
  }
  throw new Error('Unknown API error')
}

// ========== 分享 API ==========

/**
 * 创建分享链接
 */
export const createShare = async (
  request: CreateShareRequest
): Promise<ShareResponse> => {
  const response = await httpFetch(`${API_BASE}/share/create`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to create share: ${error}`)
  }

  return response.json()
}

/**
 * 列出所有分享
 */
export const listShares = async (
  fileId?: string
): Promise<ListSharesResponse> => {
  const url = fileId
    ? `${API_BASE}/share/list?fileId=${fileId}`
    : `${API_BASE}/share/list`

  const response = await httpFetch(url)

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to list shares: ${error}`)
  }

  return response.json()
}

/**
 * 撤销分享
 */
export const revokeShare = async (shareId: string): Promise<RevokeShareResponse> => {
  const response = await httpFetch(`${API_BASE}/share/revoke`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ shareId }),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to revoke share: ${error}`)
  }

  return response.json()
}

// ========== 权限 API ==========

/**
 * 设置文件权限
 */
export const setPermissions = async (
  request: SetPermissionsRequest
): Promise<SetPermissionsResponse> => {
  const response = await httpFetch(`${API_BASE}/permissions/set`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to set permissions: ${error}`)
  }

  return response.json()
}

/**
 * 获取文件权限
 */
export const getPermissions = async (filePath: string): Promise<SharePermission[]> => {
  const response = await httpFetch(
    `${API_BASE}/permissions/get?path=${encodeURIComponent(filePath)}`
  )

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to get permissions: ${error}`)
  }

  return response.json()
}

// ========== 评论 API ==========

/**
 * 获取评论列表
 */
export const listComments = async (
  filePath: string
): Promise<ListCommentsResponse> => {
  const response = await httpFetch(
    `${API_BASE}/comments?path=${encodeURIComponent(filePath)}`
  )

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to list comments: ${error}`)
  }

  return response.json()
}

/**
 * 添加评论
 */
export const addComment = async (
  request: CreateCommentRequest
): Promise<CommentResponse> => {
  const response = await httpFetch(`${API_BASE}/comments`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to add comment: ${error}`)
  }

  return response.json()
}

/**
 * 更新评论
 */
export const updateComment = async (
  commentId: string,
  request: UpdateCommentRequest
): Promise<CommentResponse> => {
  const response = await httpFetch(`${API_BASE}/comments/${commentId}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to update comment: ${error}`)
  }

  return response.json()
}

/**
 * 标记评论为已解决
 */
export const resolveComment = async (
  commentId: string
): Promise<ResolveCommentResponse> => {
  const response = await httpFetch(`${API_BASE}/comments/${commentId}/resolve`, {
    method: 'PUT',
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to resolve comment: ${error}`)
  }

  return response.json()
}

/**
 * 删除评论
 */
export const deleteComment = async (
  commentId: string
): Promise<DeleteCommentResponse> => {
  const response = await httpFetch(`${API_BASE}/comments/${commentId}`, {
    method: 'DELETE',
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to belete comment: ${error}`)
  }

  return response.json()
}

// ========== 活动历史 API ==========

/**
 * 获取活动历史
 */
export const getActivities = async (
  request: GetActivitiesRequest
): Promise<ActivitiesResponse> => {
  const params = new URLSearchParams()

  if (request.filePath) {
    params.append('path', request.filePath)
  }

  if (request.type && request.type !== 'all') {
    params.append('type', request.type)
  }

  if (request.limit) {
    params.append('limit', request.limit.toString())
  }

  if (request.offset) {
    params.append('offset', request.offset.toString())
  }

  const response = await httpFetch(
    `${API_BASE}/activities?${params.toString()}`
  )

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to get activities: ${error}`)
  }

  return response.json()
}

// ========== 用户 API ==========

/**
 * 获取用户列表
 */
export const listUsers = async (query?: string): Promise<UsersResponse> => {
  const url = query
    ? `${API_BASE}/users?query=${encodeURIComponent(query)}`
    : `${API_BASE}/users`

  const response = await httpFetch(url)

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to list users: ${error}`)
  }

  return response.json()
}

// ========== WebSocket 通知 ==========

/**
 * 创建 WebSocket 连接用于实时通知
 */
export const createNotificationSocket = (): WebSocket => {
  const wsUrl = API_BASE.replace('http', 'ws').replace('https', 'wss')
  return new WebSocket(`${wsUrl}/ws/notifications`)
}

/**
 * 订阅通知
 */
export const subscribeToNotifications = (
  onNotification: (notification: Notification) => void,
  onError?: (error: Event) => void
): (() => void) => {
  const ws = createNotificationSocket()

  ws.onopen = () => {
    console.log('Notification WebSocket connected')
  }

  ws.onmessage = (event) => {
    try {
      const notification = JSON.parse(event.data) as Notification
      onNotification(notification)
    } catch (error) {
      console.error('Failed to parse notification:', error)
    }
  }

  ws.onerror = (error) => {
    console.error('Notification WebSocket error:', error)
    onError?.(error)
  }

  ws.onclose = () => {
    console.log('Notification WebSocket closed')
  }

  // 返回清理函数
  return () => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close()
    }
  }
}

// ========== 批量操作 ==========

/**
 * 批量创建分享
 */
export const batchCreateShares = async (
  requests: CreateShareRequest[]
): Promise<ShareResponse[]> => {
  const response = await httpFetch(`${API_BASE}/share/batch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ shares: requests }),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to batch create shares: ${error}`)
  }

  return response.json()
}

/**
 * 批量删除评论
 */
export const batchDeleteComments = async (
  commentIds: string[]
): Promise<void> => {
  const response = await httpFetch(`${API_BASE}/comments/batch`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ commentIds }),
  })

  if (!response.ok) {
    const error = await response.text()
    throw new Error(`Failed to batch delete comments: ${error}`)
  }
}
