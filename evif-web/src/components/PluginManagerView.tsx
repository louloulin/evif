/**
 * PluginManagerView - 插件管理器视图
 * 核心闭环: 浏览插件 → 安装/卸载 → 配置管理 → 状态监控
 */

import React, { useState, useEffect } from 'react'
import { 
  Search, 
  Plus, 
  Download, 
  Trash2, 
  RefreshCw,
  CheckCircle,
  Plug
} from 'lucide-react'
import { 
  getAvailablePlugins,
  getMounts,
  mount,
  unmount,
} from '@/services/plugin-api'
import { LoadingSpinner, EmptyState, ErrorState } from '@/components/ui/loading'
import { toast } from '@/hooks/use-toast'

interface PluginInfo {
  name: string
  description: string
  version: string
}

const PluginManagerView: React.FC = () => {
  // ============ State ============
  const [availablePlugins, setAvailablePlugins] = useState<PluginInfo[]>([])
  const [mountedPaths, setMountedPaths] = useState<Set<string>>(new Set())
  const [searchQuery, setSearchQuery] = useState('')
  const [viewTab, setViewTab] = useState<'installed' | 'available'>('installed')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [processingNames, setProcessingNames] = useState<Set<string>>(new Set())

  // ============ Data Fetching ============
  const fetchPlugins = async () => {
    setLoading(true)
    setError(null)
    try {
      const [availableData, mountsData] = await Promise.all([
        getAvailablePlugins(),
        getMounts(),
      ])
      
      // Extract plugin names from available plugins response
      const plugins: PluginInfo[] = []
      for (const [name, info] of Object.entries(availableData)) {
        if (typeof info === 'object' && info !== null) {
          plugins.push({
            name,
            description: (info as any).description || '',
            version: (info as any).version || '1.0.0',
          })
        }
      }
      setAvailablePlugins(plugins)
      
      // Extract mounted paths
      const mounted = new Set<string>()
      for (const mount of mountsData.mounts || []) {
        mounted.add(mount.plugin)
      }
      setMountedPaths(mounted)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load plugins')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchPlugins()
  }, [])

  // ============ Handlers ============
  const handleMount = async (pluginName: string) => {
    setProcessingNames(prev => new Set(prev).add(pluginName))
    try {
      await mount(`/plugins/${pluginName}`, pluginName)
      setMountedPaths(prev => new Set(prev).add(pluginName))
      toast({
        title: 'Plugin mounted',
        description: `${pluginName} has been mounted successfully.`,
      })
    } catch (err) {
      toast({
        title: 'Mount failed',
        description: err instanceof Error ? err.message : 'Failed to mount plugin',
        variant: 'destructive',
      })
    } finally {
      setProcessingNames(prev => {
        const next = new Set(prev)
        next.delete(pluginName)
        return next
      })
    }
  }

  const handleUnmount = async (pluginName: string) => {
    if (!confirm(`Are you sure you want to unmount ${pluginName}?`)) return
    
    setProcessingNames(prev => new Set(prev).add(pluginName))
    try {
      await unmount(`/plugins/${pluginName}`)
      setMountedPaths(prev => {
        const next = new Set(prev)
        next.delete(pluginName)
        return next
      })
      toast({
        title: 'Plugin unmounted',
        description: `${pluginName} has been unmounted.`,
      })
    } catch (err) {
      toast({
        title: 'Unmount failed',
        description: err instanceof Error ? err.message : 'Failed to unmount plugin',
        variant: 'destructive',
      })
    } finally {
      setProcessingNames(prev => {
        const next = new Set(prev)
        next.delete(pluginName)
        return next
      })
    }
  }

  // ============ Filter ============
  const filteredPlugins = availablePlugins.filter(plugin => {
    // Tab filter
    const isMounted = mountedPaths.has(plugin.name)
    if (viewTab === 'installed' && !isMounted) return false
    if (viewTab === 'available' && isMounted) return false
    
    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      return (
        plugin.name.toLowerCase().includes(query) ||
        plugin.description.toLowerCase().includes(query)
      )
    }
    return true
  })

  // ============ Loading State ============
  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex flex-col items-center gap-4">
          <LoadingSpinner size="lg" />
          <p className="text-sm text-muted-foreground">Loading plugins...</p>
        </div>
      </div>
    )
  }

  // ============ Error State ============
  if (error) {
    return (
      <ErrorState error={error} onRetry={fetchPlugins} />
    )
  }

  // ============ Render ============
  return (
    <div className="plugin-manager p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Plugin Manager</h1>
          <p className="text-muted-foreground">
            {mountedPaths.size} plugins mounted
          </p>
        </div>
        <button
          onClick={fetchPlugins}
          className="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted"
        >
          <RefreshCw className="h-4 w-4" />
          Refresh
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        <button
          onClick={() => setViewTab('installed')}
          className={`px-4 py-2 text-sm font-medium border-b-2 ${
            viewTab === 'installed'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground'
          }`}
        >
          Installed ({mountedPaths.size})
        </button>
        <button
          onClick={() => setViewTab('available')}
          className={`px-4 py-2 text-sm font-medium border-b-2 ${
            viewTab === 'available'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground'
          }`}
        >
          Available ({availablePlugins.length - mountedPaths.size})
        </button>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          type="text"
          placeholder="Search plugins..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full pl-10 pr-4 py-2 border rounded-lg bg-background"
        />
      </div>

      {/* Plugin List */}
      {filteredPlugins.length === 0 ? (
        <EmptyState
          icon="folder"
          title={viewTab === 'installed' ? 'No plugins installed' : 'No plugins available'}
          description={
            viewTab === 'installed'
              ? 'Browse available plugins to get started.'
              : 'All plugins are already installed.'
          }
        />
      ) : (
        <div className="space-y-3">
          {filteredPlugins.map((plugin) => (
            <PluginCard
              key={plugin.name}
              plugin={plugin}
              isMounted={mountedPaths.has(plugin.name)}
              isProcessing={processingNames.has(plugin.name)}
              onMount={() => handleMount(plugin.name)}
              onUnmount={() => handleUnmount(plugin.name)}
            />
          ))}
        </div>
      )}
    </div>
  )
}

// ============ Plugin Card ============
interface PluginCardProps {
  plugin: PluginInfo
  isMounted: boolean
  isProcessing: boolean
  onMount: () => void
  onUnmount: () => void
}

const PluginCard: React.FC<PluginCardProps> = ({
  plugin,
  isMounted,
  isProcessing,
  onMount,
  onUnmount,
}) => {
  return (
    <div className="border rounded-lg p-4 flex items-start gap-4">
      {/* Icon */}
      <div className="p-3 bg-primary/10 rounded-lg shrink-0">
        <Plug className="h-6 w-6 text-primary" />
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="font-semibold">{plugin.name}</h3>
          {isMounted && (
            <CheckCircle className="h-4 w-4 text-green-500" />
          )}
        </div>
        <p className="text-sm text-muted-foreground mt-1">{plugin.description}</p>
        <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
          <span>v{plugin.version}</span>
        </div>
      </div>

      {/* Actions */}
      <div className="shrink-0">
        {isProcessing ? (
          <LoadingSpinner size="sm" />
        ) : isMounted ? (
          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1 text-sm text-green-500">
              <CheckCircle className="h-4 w-4" />
              Mounted
            </span>
            <button
              onClick={onUnmount}
              className="p-2 text-destructive hover:bg-destructive/10 rounded"
              title="Unmount"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        ) : (
          <button
            onClick={onMount}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            <Download className="h-4 w-4" />
            Mount
          </button>
        )}
      </div>
    </div>
  )
}

export default PluginManagerView
