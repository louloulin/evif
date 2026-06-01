/**
 * Admin API 服务
 * 对接 /api/v1/admin/* 端点
 */

import { httpFetch } from '@/lib/http'

// ============ Types ============

export interface OverviewStats {
  total_tenants: number
  total_plugins: number
  total_revenue_cents: number
  total_api_calls: number
  active_tenants: number
  growth_rate: number
}

export interface TenantStat {
  id: string
  name: string
  storage_used: number
  storage_quota: number
  api_calls: number
  plugins_count: number
  created_at: string
}

export interface PluginStat {
  id: string
  name: string
  downloads: number
  rating: number
  revenue_cents: number
  author: string
}

export interface RevenueStat {
  date: string
  amount_cents: number
  subscriptions: number
  plugins: number
}

export interface AuditLogEntry {
  id: string
  tenant_id: string
  user_id: string
  action: string
  resource: string
  timestamp: string
  ip_address: string
  details: Record<string, unknown>
}

export interface AuditLogParams {
  tenant_id?: string
  user_id?: string
  action?: string
  start_date?: string
  end_date?: string
  limit?: number
  offset?: number
}

export interface User {
  id: string
  email: string
  name: string
  role: 'admin' | 'user' | 'viewer'
  tenant_id: string
  created_at: string
  last_login: string
  status: 'active' | 'suspended' | 'deleted'
}

export interface CreateUserRequest {
  email: string
  name: string
  role?: 'admin' | 'user' | 'viewer'
  tenant_id: string
}

// ============ API Functions ============

/**
 * 获取全局概览统计
 */
export async function getOverview(): Promise<OverviewStats> {
  const res = await httpFetch('/api/v1/admin/stats/overview')
  if (!res.ok) throw new Error('Failed to fetch overview')
  return res.json()
}

/**
 * 获取租户统计
 */
export async function getTenantStats(): Promise<TenantStat[]> {
  const res = await httpFetch('/api/v1/admin/stats/tenants')
  if (!res.ok) throw new Error('Failed to fetch tenant stats')
  return res.json()
}

/**
 * 获取插件统计
 */
export async function getPluginStats(): Promise<PluginStat[]> {
  const res = await httpFetch('/api/v1/admin/stats/plugins')
  if (!res.ok) throw new Error('Failed to fetch plugin stats')
  return res.json()
}

/**
 * 获取收入统计
 */
export async function getRevenueStats(params?: { months?: number }): Promise<RevenueStat[]> {
  const url = params?.months 
    ? `/api/v1/admin/stats/revenue?months=${params.months}`
    : '/api/v1/admin/stats/revenue'
  const res = await httpFetch(url)
  if (!res.ok) throw new Error('Failed to fetch revenue stats')
  return res.json()
}

/**
 * 获取审计日志
 */
export async function getAuditLog(params?: AuditLogParams): Promise<AuditLogEntry[]> {
  const searchParams = new URLSearchParams()
  if (params?.tenant_id) searchParams.set('tenant_id', params.tenant_id)
  if (params?.user_id) searchParams.set('user_id', params.user_id)
  if (params?.action) searchParams.set('action', params.action)
  if (params?.start_date) searchParams.set('start_date', params.start_date)
  if (params?.end_date) searchParams.set('end_date', params.end_date)
  if (params?.limit) searchParams.set('limit', String(params.limit))
  if (params?.offset) searchParams.set('offset', String(params.offset))
  
  const url = `/api/v1/admin/audit${searchParams.toString() ? '?' + searchParams.toString() : ''}`
  const res = await httpFetch(url)
  if (!res.ok) throw new Error('Failed to fetch audit log')
  return res.json()
}

/**
 * 获取用户列表
 */
export async function listUsers(): Promise<User[]> {
  const res = await httpFetch('/api/v1/admin/users')
  if (!res.ok) throw new Error('Failed to fetch users')
  return res.json()
}

/**
 * 创建用户
 */
export async function createUser(data: CreateUserRequest): Promise<User> {
  const res = await httpFetch('/api/v1/admin/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to create user')
  return res.json()
}

/**
 * 获取用户详情
 */
export async function getUser(id: string): Promise<User> {
  const res = await httpFetch(`/api/v1/admin/users/${id}`)
  if (!res.ok) throw new Error('Failed to fetch user')
  return res.json()
}

/**
 * 更新用户
 */
export async function updateUser(id: string, data: Partial<User>): Promise<User> {
  const res = await httpFetch(`/api/v1/admin/users/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to update user')
  return res.json()
}

/**
 * 删除用户
 */
export async function deleteUser(id: string): Promise<void> {
  const res = await httpFetch(`/api/v1/admin/users/${id}`, {
    method: 'DELETE',
  })
  if (!res.ok) throw new Error('Failed to delete user')
}

// ============ Helper Functions ============

export function formatCents(cents: number): string {
  return `$${(cents / 100).toFixed(2)}`
}

export function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`
  return String(num)
}
