/**
 * Phase 9.2: 监控 API 服务
 * 对接 GET /api/v1/metrics/status、/api/v1/metrics/traffic、/api/v1/metrics/operations
 */

import { httpFetch } from '@/lib/http'

export interface MetricsStatusResponse {
  status: string
  uptime_secs?: number
  uptime?: number
  mounts?: { count: number; list?: string[] }
  traffic?: {
    total_requests: number
    total_bytes_read: number
    total_bytes_written: number
    total_errors: number
    read_count: number
    write_count: number
    list_count: number
    other_count: number
    average_read_size: number
    average_write_size: number
  }
  operations?: Array<{ operation: string; count: number; bytes: number; errors: number }>
}

export async function getMetricsStatus(): Promise<MetricsStatusResponse> {
  const res = await httpFetch('/api/v1/metrics/status')
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Failed to fetch metrics status')
  }
  return res.json()
}

export async function getMetricsTraffic(): Promise<{
  total_requests: number
  total_bytes_read: number
  total_bytes_written: number
  total_errors: number
  read_count: number
  write_count: number
  list_count: number
  other_count: number
  average_read_size: number
  average_write_size: number
}> {
  const res = await httpFetch('/api/v1/metrics/traffic')
  if (!res.ok) throw new Error('Failed to fetch traffic')
  return res.json()
}

export async function getMetricsOperations(): Promise<
  Array<{ operation: string; count: number; bytes: number; errors: number }>
> {
  const res = await httpFetch('/api/v1/metrics/operations')
  if (!res.ok) throw new Error('Failed to fetch operations')
  return res.json()
}
