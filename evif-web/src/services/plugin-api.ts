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

export interface AvailablePluginInfo {
  id: string
  name: string
  display_name: string
  version: string
  description: string
  type: 'local' | 'cloud-storage' | 'ai' | 'database' | 'other'
  support_tier: 'core' | 'dynamic' | 'experimental' | string
  is_mountable: boolean
  is_loaded: boolean
  is_mounted: boolean
  mount_path?: string
  aliases?: string[]
}

export interface AvailablePluginsResponse {
  plugins: AvailablePluginInfo[]
  total: number
}

/** 插件名到 API 名称的映射（后端接受 mem | memfs 等） */
export function toApiPluginName(id: string): string {
  const lower = id.toLowerCase()
  if (lower === 'memfs') return 'mem'
  if (lower === 'hellofs') return 'hello'
  if (lower === 'localfs') return 'local'
  if (lower === 'sqlfs2') return 'sqlfs2'
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

export async function getAvailablePlugins(): Promise<AvailablePluginsResponse> {
  const res = await httpFetch('/api/v1/plugins/available')
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Failed to fetch available plugins')
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
