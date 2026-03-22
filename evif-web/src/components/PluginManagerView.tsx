/**
 * Phase 9.1: 插件管理视图，对接真实 API
 * GET /api/v1/mounts、POST /api/v1/mount、POST /api/v1/unmount、
 * GET /api/v1/plugins/:name/readme、GET /api/v1/plugins/:name/config
 */

import React, { useState, useEffect, useCallback } from 'react'
import { PluginList } from '@/components/plugin-manager/PluginList'
import { MountModal } from '@/components/plugin-manager/MountModal'
import { PluginModal } from '@/components/plugin-manager/PluginModal'
import type { Plugin, MountPoint, PluginConfig } from '@/types/plugin'
import {
  getAvailablePlugins,
  getMounts,
  mount as apiMount,
  unmount as apiUnmount,
  getPluginReadme,
  getPluginConfig,
  type AvailablePluginInfo,
} from '@/services/plugin-api'

function normalizePluginId(name: string): string {
  const lower = name.toLowerCase()
  if (lower === 'mem') return 'memfs'
  if (lower === 'hello') return 'hellofs'
  if (lower === 'local') return 'localfs'
  if (lower === 'sqlfs') return 'sqlfs2'
  return lower
}

function buildPlugins(
  availablePlugins: AvailablePluginInfo[],
  mounts: { path: string; plugin: string }[]
): Plugin[] {
  const mountedByPlugin = new Map<string, string>()
  for (const mount of mounts) {
    mountedByPlugin.set(normalizePluginId(mount.plugin), mount.path)
  }

  return availablePlugins.map((plugin) => {
    const mountPoint = plugin.mount_path ?? mountedByPlugin.get(normalizePluginId(plugin.id))
    const status: Plugin['status'] =
      mountPoint ? 'loaded' : plugin.support_tier === 'dynamic' && plugin.is_loaded ? 'loaded' : 'unloaded'
    return {
      id: plugin.id,
      name: plugin.display_name || plugin.name,
      version: plugin.version,
      author:
        plugin.support_tier === 'dynamic'
          ? 'Dynamic Plugin'
          : plugin.support_tier === 'experimental'
            ? 'Experimental'
            : 'EVIF Core',
      description: plugin.description,
      type: plugin.type,
      supportTier: plugin.support_tier,
      mountable: plugin.is_mountable,
      status,
      mountPoint,
      capabilities: ['read', 'write'],
    }
  }).sort((left, right) => {
    const rank = (tier: string) => tier === 'core' ? 0 : tier === 'dynamic' ? 1 : 2
    return rank(left.supportTier) - rank(right.supportTier) || left.name.localeCompare(right.name)
  })
}

export const PluginManagerView: React.FC = () => {
  const [plugins, setPlugins] = useState<Plugin[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [mountModalPlugin, setMountModalPlugin] = useState<Plugin | null>(null)
  const [configModalPlugin, setConfigModalPlugin] = useState<Plugin | null>(null)
  const [readmeContent, setReadmeContent] = useState('')
  const [configParams, setConfigParams] = useState<PluginConfig>({})

  const fetchMounts = useCallback(async () => {
    try {
      setError(null)
      const [available, mounts] = await Promise.all([
        getAvailablePlugins(),
        getMounts(),
      ])
      setPlugins(buildPlugins(available.plugins, mounts.mounts))
    } catch (e) {
      setError(e instanceof Error ? e.message : '加载挂载点失败')
      setPlugins([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchMounts()
  }, [fetchMounts])

  const existingMounts: MountPoint[] = plugins
    .filter((p) => p.mountPoint)
    .map((p) => ({ path: p.mountPoint!, pluginId: p.id, mounted: true }))

  const handleMount = useCallback(
    async (pluginId: string, mountPoint: string, options?: { readOnly?: boolean }) => {
      const config = pluginId === 'localfs' ? { root: '/tmp/evif-local' } : undefined
      await apiMount(mountPoint, pluginId, config)
      setMountModalPlugin(null)
      await fetchMounts()
    },
    [fetchMounts]
  )

  const handleUnmount = useCallback(
    async (pluginId: string) => {
      const p = plugins.find((x) => x.id === pluginId && x.mountPoint)
      if (!p?.mountPoint) return
      await apiUnmount(p.mountPoint)
      await fetchMounts()
    },
    [plugins, fetchMounts]
  )

  const handleOpenConfig = useCallback(async (pluginId: string) => {
    const p = plugins.find((x) => x.id === pluginId)
    if (!p) return
    setConfigModalPlugin(p)
    try {
      const [readmeRes, configRes] = await Promise.all([
        getPluginReadme(pluginId),
        getPluginConfig(pluginId),
      ])
      setReadmeContent(readmeRes.readme)
      const cfg: PluginConfig = {}
      for (const param of configRes.params) {
        if (param.default != null) cfg[param.name] = param.default
      }
      setConfigParams(cfg)
    } catch {
      setReadmeContent('(加载README失败)')
      setConfigParams({})
    }
  }, [plugins])

  if (loading) {
    return (
      <div className="p-4 md:p-6 lg:p-8 text-muted-foreground text-sm md:text-base">
        加载挂载点中...
      </div>
    )
  }

  return (
    <div className="p-4 md:p-6 lg:p-8 h-full overflow-auto">
      {error && (
        <div className="mb-4 md:mb-6 p-3 md:p-4 rounded-md bg-destructive/10 text-destructive text-sm md:text-base">
          {error}
        </div>
      )}
      <PluginList
        plugins={plugins}
        onPluginMount={(id) => setMountModalPlugin(plugins.find((p) => p.id === id) ?? null)}
        onPluginConfigure={handleOpenConfig}
        onPluginToggle={(id, load) => (load ? setMountModalPlugin(plugins.find((p) => p.id === id) ?? null) : handleUnmount(id))}
      />
      <MountModal
        plugin={mountModalPlugin}
        open={!!mountModalPlugin}
        onClose={() => setMountModalPlugin(null)}
        onMount={handleMount}
        existingMounts={existingMounts}
      />
      <PluginModal
        plugin={configModalPlugin}
        open={!!configModalPlugin}
        onClose={() => setConfigModalPlugin(null)}
        onSave={async () => setConfigModalPlugin(null)}
        readmeOverride={readmeContent}
        configParamsOverride={configParams}
      />
    </div>
  )
}

export default PluginManagerView
