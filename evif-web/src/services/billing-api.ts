/**
 * Billing API 服务
 * 对接 /api/v1/billing/* 端点
 */

import { httpFetch } from '@/lib/http'

// ============ Types ============

export type PricingPlan = 'free' | 'pro' | 'team' | 'enterprise'

export interface Quota {
  used: number
  limit: number
  unit: string
}

export interface UsageStats {
  api_calls: Quota
  storage: Quota
  agents: Quota
}

export interface BillingPeriod {
  start: string
  end: string
}

export interface UsageResponse {
  tenant_id: string
  period: BillingPeriod
  plan: PricingPlan
  quotas: UsageStats
}

export interface UsageHistoryEntry {
  date: string
  api_calls: number
  storage_bytes: number
  active_agents: number
}

export interface UsageDetail {
  endpoint: string
  count: number
  total_latency_ms: number
}

export interface Subscription {
  tenant_id: string
  plan: PricingPlan
  started_at: string
  next_billing_at: string
  status: 'active' | 'trial' | 'cancelled' | 'past_due'
}

export interface CreateSubscriptionRequest {
  plan: PricingPlan
}

export interface UpdateSubscriptionRequest {
  plan: PricingPlan
}

export interface Invoice {
  id: string
  tenant_id: string
  period: BillingPeriod
  amount_cents: number
  status: 'draft' | 'paid' | 'unpaid' | 'void'
  created_at: string
  pdf_url?: string
}

export interface WebhookConfig {
  id: string
  tenant_id: string
  url: string
  events: WebhookEvent[]
  secret: string
}

export type WebhookEvent = 
  | 'quota_80_percent' 
  | 'quota_100_percent' 
  | 'subscription_changed' 
  | 'invoice_created'

export interface CreateWebhookRequest {
  url: string
  events: WebhookEvent[]
}

export interface StripeSessionResponse {
  session_id: string
  url: string
}

// ============ API Functions ============

/**
 * 获取当前使用量
 */
export async function getUsage(): Promise<UsageResponse> {
  const res = await httpFetch('/api/v1/billing/usage')
  if (!res.ok) throw new Error('Failed to fetch usage')
  return res.json()
}

/**
 * 获取历史使用量
 */
export async function getUsageHistory(months: number = 12): Promise<UsageHistoryEntry[]> {
  const res = await httpFetch(`/api/v1/billing/usage/history?months=${months}`)
  if (!res.ok) throw new Error('Failed to fetch usage history')
  return res.json()
}

/**
 * 按端点获取使用量
 */
export async function getUsageByEndpoint(): Promise<UsageDetail[]> {
  const res = await httpFetch('/api/v1/billing/usage/by-endpoint')
  if (!res.ok) throw new Error('Failed to fetch usage by endpoint')
  return res.json()
}

/**
 * 获取当前订阅
 */
export async function getSubscription(): Promise<Subscription> {
  const res = await httpFetch('/api/v1/billing/subscription')
  if (!res.ok) throw new Error('Failed to fetch subscription')
  return res.json()
}

/**
 * 创建订阅
 */
export async function createSubscription(data: CreateSubscriptionRequest): Promise<Subscription> {
  const res = await httpFetch('/api/v1/billing/subscription', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to create subscription')
  return res.json()
}

/**
 * 更新订阅 (升级/降级)
 */
export async function updateSubscription(data: UpdateSubscriptionRequest): Promise<Subscription> {
  const res = await httpFetch('/api/v1/billing/subscription', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to update subscription')
  return res.json()
}

/**
 * 获取发票列表
 */
export async function getInvoices(): Promise<Invoice[]> {
  const res = await httpFetch('/api/v1/billing/invoices')
  if (!res.ok) throw new Error('Failed to fetch invoices')
  return res.json()
}

/**
 * 获取发票详情
 */
export async function getInvoice(id: string): Promise<Invoice> {
  const res = await httpFetch(`/api/v1/billing/invoices/${id}`)
  if (!res.ok) throw new Error('Failed to fetch invoice')
  return res.json()
}

/**
 * 创建 Stripe Checkout Session
 */
export async function createCheckout(params: {
  plan: PricingPlan
  success_url: string
  cancel_url: string
}): Promise<StripeSessionResponse> {
  const res = await httpFetch('/api/v1/billing/stripe/checkout', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  })
  if (!res.ok) throw new Error('Failed to create checkout session')
  return res.json()
}

/**
 * 创建 Stripe Customer Portal Session
 */
export async function createPortal(params: { return_url: string }): Promise<StripeSessionResponse> {
  const res = await httpFetch('/api/v1/billing/stripe/portal', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  })
  if (!res.ok) throw new Error('Failed to create portal session')
  return res.json()
}

/**
 * 获取 Webhook 配置列表
 */
export async function getWebhooks(): Promise<WebhookConfig[]> {
  const res = await httpFetch('/api/v1/billing/webhooks')
  if (!res.ok) throw new Error('Failed to fetch webhooks')
  return res.json()
}

/**
 * 创建 Webhook 配置
 */
export async function createWebhook(data: CreateWebhookRequest): Promise<WebhookConfig> {
  const res = await httpFetch('/api/v1/billing/webhooks', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!res.ok) throw new Error('Failed to create webhook')
  return res.json()
}

/**
 * 删除 Webhook 配置
 */
export async function deleteWebhook(id: string): Promise<void> {
  const res = await httpFetch(`/api/v1/billing/webhooks/${id}`, {
    method: 'DELETE',
  })
  if (!res.ok) throw new Error('Failed to delete webhook')
}

// ============ Helper Functions ============

/**
 * 计算配额使用百分比
 */
export function getUsagePercent(quota: Quota): number {
  if (quota.limit === 0) return 0
  return Math.round((quota.used / quota.limit) * 100 * 10) / 10
}

/**
 * 格式化配额显示
 */
export function formatQuota(quota: Quota): string {
  const percent = getUsagePercent(quota)
  const used = formatBytes(quota.used)
  const limit = formatBytes(quota.limit)
  return `${used} / ${limit} (${percent}%)`
}

/**
 * 格式化字节数
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}

/**
 * 格式化金额
 */
export function formatCents(cents: number): string {
  return `$${(cents / 100).toFixed(2)}`
}
