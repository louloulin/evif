/**
 * Marketplace API 服务
 * 对接 /api/v1/marketplace/* 端点
 */

import { httpFetch } from '@/lib/http'

// ============ Types ============

export interface Plugin {
  id: string
  name: string
  description: string
  version: string
  author: string
  author_id: string
  category: string
  tags: string[]
  downloads: number
  rating: number
  rating_count: number
  price_cents: number
  is_free: boolean
  verified: boolean
  created_at: string
  updated_at: string
  thumbnail_url?: string
  homepage_url?: string
  repository_url?: string
}

export interface PluginDetail extends Plugin {
  long_description: string
  screenshots: string[]
  changelog: string
  requirements: string[]
  compatibility: Record<string, string>
  support_email?: string
  license: string
  total_revenue: number
}

export interface PluginListParams {
  category?: string
  search?: string
  sort?: 'downloads' | 'rating' | 'recent' | 'price'
  order?: 'asc' | 'desc'
  page?: number
  limit?: number
  tier?: 'free' | 'paid' | 'all'
}

export interface PluginListResponse {
  plugins: Plugin[]
  total: number
  page: number
  limit: number
  total_pages: number
}

export interface RatePluginRequest {
  rating: number
  comment?: string
}

export interface PublishPluginRequest {
  name: string
  description: string
  long_description?: string
  version: string
  category: string
  tags: string[]
  price_cents?: number
  homepage_url?: string
  repository_url?: string
  requirements?: string[]
  license?: string
}

// ============ API Functions ============

/**
 * 获取插件列表
 */
export async function listPlugins(params?: PluginListParams): Promise<PluginListResponse> {
  const searchParams = new URLSearchParams()
  if (params?.category) searchParams.set('category', params.category)
  if (params?.search) searchParams.set('search', params.search)
  if (params?.sort) searchParams.set('sort', params.sort)
  if (params?.order) searchParams.set('order', params.order)
  if (params?.page) searchParams.set('page', String(params.page))
  if (params?.limit) searchParams.set('limit', String(params.limit))
  if (params?.tier) searchParams.set('tier', params.tier)
  
  const url = `/api/v1/marketplace/plugins${searchParams.toString() ? '?' + searchParams.toString() : ''}`
  const res = await httpFetch(url)
  if (!res.ok) throw new Error('Failed to fetch plugins')
  return res.json()
}

/**
 * 获取插件详情
 */
export async function getPlugin(id: string): Promise<PluginDetail> {
  const res = await httpFetch(`/api/v1/marketplace/plugins/${id}`)
  if (!res.ok) throw new Error('Failed to fetch plugin')
  return res.json()
}

/**
 * 更新插件
 */
export async function updatePlugin(id: string, data: Partial<Plugin>): Promise<Plugin> {
  const res = await httpFetch(`/api/v1/marketplace/plugins/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to update plugin')
  return res.json()
}

/**
 * 发布插件
 */
export async function publishPlugin(data: PublishPluginRequest): Promise<Plugin> {
  const res = await httpFetch('/api/v1/marketplace/plugins', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to publish plugin')
  return res.json()
}

/**
 * 增加下载计数
 */
export async function incrementDownload(id: string): Promise<void> {
  const res = await httpFetch(`/api/v1/marketplace/plugins/${id}/download`, {
    method: 'POST',
  })
  if (!res.ok) throw new Error('Failed to increment download')
}

/**
 * 评分插件
 */
export async function ratePlugin(id: string, data: RatePluginRequest): Promise<void> {
  const res = await httpFetch(`/api/v1/marketplace/plugins/${id}/rate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to rate plugin')
}

/**
 * 获取热门插件
 */
export async function getTrending(limit: number = 10): Promise<Plugin[]> {
  const res = await httpFetch(`/api/v1/marketplace/trending?limit=${limit}`)
  if (!res.ok) throw new Error('Failed to fetch trending plugins')
  return res.json()
}

/**
 * 获取免费插件
 */
export async function getFreePlugins(params?: { limit?: number }): Promise<Plugin[]> {
  const url = `/api/v1/marketplace/free${params?.limit ? '?limit=' + params.limit : ''}`
  const res = await httpFetch(url)
  if (!res.ok) throw new Error('Failed to fetch free plugins')
  return res.json()
}

// ============ Helper Functions ============

/**
 * 格式化下载量
 */
export function formatDownloads(count: number): string {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M`
  }
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}K`
  }
  return String(count)
}

/**
 * 格式化价格
 */
export function formatPrice(cents: number): string {
  if (cents === 0) return 'Free'
  return `$${(cents / 100).toFixed(2)}`
}

/**
 * 获取评分星星
 */
export function getRatingStars(rating: number): { full: number; half: boolean; empty: number } {
  const full = Math.floor(rating)
  const half = rating - full >= 0.5
  const empty = 5 - full - (half ? 1 : 0)
  return { full, half, empty }
}
