import React, { useState, useEffect, useRef } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Terminal, Download, Trash2, RefreshCw } from 'lucide-react'

interface LogEntry {
  timestamp: string
  level: 'info' | 'warn' | 'error' | 'debug'
  message: string
  plugin?: string
}

interface PluginLogsProps {
  pluginId: string
  pluginName: string
  autoRefresh?: boolean
}

const LOG_LEVEL_COLORS = {
  'info': 'text-blue-500',
  'warn': 'text-yellow-500',
  'error': 'text-red-500',
  'debug': 'text-gray-500',
}

const LOG_LEVEL_BADGES = {
  'info': 'default',
  'warn': 'secondary',
  'error': 'destructive',
  'debug': 'outline',
} as const

export const PluginLogs: React.FC<PluginLogsProps> = ({
  pluginId,
  pluginName,
  autoRefresh = true,
}) => {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [loading, setLoading] = useState(false)
  const scrollRef = useRef<HTMLDivElement>(null)

  // Mock initial logs
  useEffect(() => {
    const mockLogs: LogEntry[] = [
      {
        timestamp: new Date().toISOString(),
        level: 'info',
        message: `Plugin ${pluginName} loaded successfully`,
        plugin: pluginName,
      },
      {
        timestamp: new Date(Date.now() - 1000).toISOString(),
        level: 'debug',
        message: 'Initializing plugin capabilities...',
        plugin: pluginName,
      },
      {
        timestamp: new Date(Date.now() - 2000).toISOString(),
        level: 'info',
        message: '正在从 /etc/evif/plugins/config.json 加载配置',
        plugin: pluginName,
      },
    ]
    setLogs(mockLogs)
  }, [pluginId, pluginName])

  const handleRefresh = async () => {
    setLoading(true)
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 500))
    setLoading(false)
  }

  const handleClear = () => {
    setLogs([])
  }

  const handleExport = () => {
    const logText = logs.map(log =>
      `[${log.timestamp}] [${log.level.toUpperCase()}] ${log.message}`
    ).join('\n')

    const blob = new Blob([logText], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${pluginName}-logs-${new Date().toISOString()}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Terminal className="h-5 w-5" />
            <CardTitle>Plugin Logs</CardTitle>
            <Badge variant="outline">{pluginName}</Badge>
          </div>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={handleRefresh}
              disabled={loading}
            >
              <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleExport}
            >
              <Download className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleClear}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden">
        <ScrollArea className="h-full" ref={scrollRef}>
          <div className="space-y-1 font-mono text-xs">
            {logs.length === 0 ? (
              <div className="text-center text-muted-foreground py-8">
                No logs available
              </div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="flex gap-2 py-1 hover:bg-muted/50 px-2 rounded">
                  <span className="text-muted-foreground shrink-0">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                  <Badge
                    variant={LOG_LEVEL_BADGES[log.level]}
                    className="shrink-0 h-5 px-1"
                  >
                    {log.level.toUpperCase()}
                  </Badge>
                  <span className={LOG_LEVEL_COLORS[log.level]}>
                    {log.message}
                  </span>
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  )
}

export default PluginLogs
