import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Plugin } from '@/types/plugin'
import {
  HardDrive,
  Cloud,
  Brain,
  Database,
  Puzzle,
  CheckCircle,
  XCircle,
  Loader2,
  Power,
  Settings,
  Folder
} from 'lucide-react'

const PLUGIN_ICONS = {
  'local': HardDrive,
  'cloud-storage': Cloud,
  'ai': Brain,
  'database': Database,
  'other': Puzzle,
}

const STATUS_ICONS = {
  'loaded': CheckCircle,
  'unloaded': XCircle,
  'error': XCircle,
}

const STATUS_COLORS = {
  'loaded': 'text-green-500',
  'unloaded': 'text-gray-500',
  'error': 'text-red-500',
}

const STATUS_LABELS: Record<string, string> = {
  'loaded': '已加载',
  'unloaded': '未加载',
  'error': '错误',
}

const SUPPORT_TIER_LABELS: Record<string, string> = {
  core: '核心支持',
  dynamic: '动态加载',
  experimental: '实验性',
}

interface PluginListProps {
  plugins: Plugin[]
  onPluginToggle?: (pluginId: string, load: boolean) => void
  onPluginConfigure?: (pluginId: string) => void
  onPluginMount?: (pluginId: string) => void
}

export const PluginList: React.FC<PluginListProps> = ({
  plugins,
  onPluginToggle,
  onPluginConfigure,
  onPluginMount,
}) => {
  const [filter, setFilter] = useState<'all' | 'loaded' | 'unloaded'>('all')
  const [search, setSearch] = useState('')

  // Group plugins by type
  const pluginsByType = plugins.reduce((acc, plugin) => {
    if (!acc[plugin.type]) {
      acc[plugin.type] = []
    }
    acc[plugin.type].push(plugin)
    return acc
  }, {} as Record<string, Plugin[]>)

  // Filter plugins
  const filteredPlugins = plugins.filter(plugin => {
    const matchesFilter = filter === 'all' || plugin.status === filter
    const matchesSearch = plugin.name.toLowerCase().includes(search.toLowerCase()) ||
                         plugin.description.toLowerCase().includes(search.toLowerCase())
    return matchesFilter && matchesSearch
  })

  const PluginCard: React.FC<{ plugin: Plugin }> = ({ plugin }) => {
    const Icon = PLUGIN_ICONS[plugin.type] || Puzzle
    const StatusIcon = STATUS_ICONS[plugin.status]
    const supportTier = plugin.supportTier ?? 'experimental'
    const mountable = plugin.mountable ?? true

    return (
      <Card className="hover:shadow-md transition-shadow">
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <Icon className="h-5 w-5 text-primary" />
              <div>
                <CardTitle className="text-lg">{plugin.name}</CardTitle>
                <CardDescription className="text-xs mt-1">
                  v{plugin.version} by {plugin.author}
                </CardDescription>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <StatusIcon className={`h-4 w-4 ${STATUS_COLORS[plugin.status]}`} />
              <Badge variant={plugin.status === 'loaded' ? 'default' : 'secondary'}>
                {STATUS_LABELS[plugin.status] || plugin.status}
              </Badge>
              <Badge variant="outline">
                {SUPPORT_TIER_LABELS[supportTier] || supportTier}
              </Badge>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground line-clamp-2">
            {plugin.description}
          </p>

          {plugin.capabilities && plugin.capabilities.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {plugin.capabilities.slice(0, 3).map((cap, i) => (
                <Badge key={i} variant="outline" className="text-xs">
                  {cap}
                </Badge>
              ))}
              {plugin.capabilities.length > 3 && (
                <Badge variant="outline" className="text-xs">
                  +{plugin.capabilities.length - 3}
                </Badge>
              )}
            </div>
          )}

          <div className="flex gap-2 pt-2">
            {plugin.status === 'loaded' ? (
              <>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => onPluginMount?.(plugin.id)}
                  className="flex-1"
                  disabled={!mountable}
                >
                  <Folder className="h-4 w-4 mr-1" />
                  挂载
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => onPluginConfigure?.(plugin.id)}
                >
                  <Settings className="h-4 w-4" />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => onPluginToggle?.(plugin.id, false)}
                >
                  <Power className="h-4 w-4" />
                </Button>
              </>
            ) : (
              <Button
                size="sm"
                onClick={() => onPluginToggle?.(plugin.id, true)}
                className="flex-1"
                disabled={!mountable}
              >
                <Power className="h-4 w-4 mr-1" />
                {mountable ? '加载插件' : '需后端装配'}
              </Button>
            )}
          </div>

          {plugin.mountPoint && (
            <div className="text-xs text-muted-foreground mt-2">
              <span className="font-medium">挂载于:</span> {plugin.mountPoint}
            </div>
          )}
          {!mountable && (
            <div className="text-xs text-muted-foreground mt-2">
              该插件需要额外后端装配，当前 UI 不直接提供一键挂载。
            </div>
          )}
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">插件管理器</h2>
          <p className="text-sm text-muted-foreground">
            管理 {plugins.length} 个可用插件
          </p>
        </div>
      </div>

      {/* Search and Filter */}
      <div className="flex gap-4">
        <input
          type="text"
          placeholder="搜索插件..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 px-4 py-2 rounded-md border bg-background"
        />
        <Tabs value={filter} onValueChange={(v) => setFilter(v as any)}>
          <TabsList>
            <TabsTrigger value="all">全部 ({plugins.length})</TabsTrigger>
            <TabsTrigger value="loaded">
              已加载 ({plugins.filter(p => p.status === 'loaded').length})
            </TabsTrigger>
            <TabsTrigger value="unloaded">
              未加载 ({plugins.filter(p => p.status === 'unloaded').length})
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {/* Plugin Grid by Type */}
      <Tabs defaultValue="local">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="local">
            <HardDrive className="h-4 w-4 mr-2" />
            本地
          </TabsTrigger>
          <TabsTrigger value="cloud-storage">
            <Cloud className="h-4 w-4 mr-2" />
            云端
          </TabsTrigger>
          <TabsTrigger value="ai">
            <Brain className="h-4 w-4 mr-2" />
            AI
          </TabsTrigger>
          <TabsTrigger value="database">
            <Database className="h-4 w-4 mr-2" />
            数据库
          </TabsTrigger>
          <TabsTrigger value="other">
            <Puzzle className="h-4 w-4 mr-2" />
            其他
          </TabsTrigger>
        </TabsList>

        {(Object.keys(pluginsByType) as Array<keyof typeof PLUGIN_ICONS>).map(type => (
          <TabsContent key={type} value={type} className="mt-4">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {pluginsByType[type]
                ?.filter(p =>
                  (filter === 'all' || p.status === filter) &&
                  (p.name.toLowerCase().includes(search.toLowerCase()) ||
                   p.description.toLowerCase().includes(search.toLowerCase()))
                )
                .map(plugin => (
                  <PluginCard key={plugin.id} plugin={plugin} />
                ))}
            </div>
            {(!pluginsByType[type] || pluginsByType[type].length === 0) && (
              <div className="text-center py-12 text-muted-foreground">
                未找到{type === 'local' ? '本地' : type === 'cloud-storage' ? '云端' : type === 'ai' ? 'AI' : type === 'database' ? '数据库' : '其他'}类型插件
              </div>
            )}
          </TabsContent>
        ))}
      </Tabs>
    </div>
  )
}

export default PluginList
