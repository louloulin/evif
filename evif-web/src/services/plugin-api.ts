/**
 * Phase 9.1: 插件与挂载 API 服务
 * 对接 GET /api/v1/mounts、POST /api/v1/mount、POST /api/v1/unmount、
 * GET /api/v1/plugins/:name/readme、GET /api/v1/plugins/:name/config
 */

import { httpFetch } from '@/lib/http'

export interface MountInfo {
  path: string
  plugin: string
}

export interface MountsResponse {
  mounts: MountInfo[]
}

export interface PluginReadmeResponse {
  name: string
  readme: string
}

export interface PluginConfigParam {
  name: string
  param_type: string
  required: boolean
  default?: string
  description?: string
}

export interface PluginConfigResponse {
  name: string
  params: PluginConfigParam[]
}

/** 后端支持的插件名（用于创建实例），与 evif-rest handlers 一致 */
export const KNOWN_PLUGIN_IDS = ['memfs', 'hellofs', 'localfs'] as const

/** 插件名到 API 名称的映射（后端接受 mem | memfs 等） */
export function toApiPluginName(id: string): string {
  const lower = id.toLowerCase()
  if (lower === 'memfs') return 'mem'
  if (lower === 'hellofs') return 'hello'
  if (lower === 'localfs') return 'local'
  return id
}

export async function getMounts(): Promise<MountsResponse> {
  const res = await httpFetch('/api/v1/mounts')
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Failed to fetch mounts')
  }
  return res.json()
}

export async function mount(path: string, plugin: string, config?: Record<string, unknown>): Promise<void> {
  const apiName = toApiPluginName(plugin)
  const res = await httpFetch('/api/v1/mount', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, plugin: apiName, config: config ?? null }),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    const msg = (data && (data.message || data.error)) || 'Mount failed'
    throw new Error(msg)
  }
}

export async function unmount(path: string): Promise<void> {
  const res = await httpFetch('/api/v1/unmount', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    const msg = (data && (data.message || data.error)) || 'Unmount failed'
    throw new Error(msg)
  }
}

export async function getPluginReadme(pluginName: string): Promise<PluginReadmeResponse> {
  const name = toApiPluginName(pluginName)
  const res = await httpFetch(`/api/v1/plugins/${encodeURIComponent(name)}/readme`)
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Failed to fetch readme')
  }
  return res.json()
}

export async function getPluginConfig(pluginName: string): Promise<PluginConfigResponse> {
  const name = toApiPluginName(pluginName)
  const res = await httpFetch(`/api/v1/plugins/${encodeURIComponent(name)}/config`)
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Failed to fetch config')
  }
  return res.json()
}
