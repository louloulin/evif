import React, { useState, useEffect, useRef } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { FileText, Download, Trash2, RefreshCw, Search } from 'lucide-react'
import { Input } from '@/components/ui/input'
import type { LogEntry } from '@/types/monitor'

interface LogViewerProps {
  logs?: LogEntry[]
  autoScroll?: boolean
}

export const LogViewer: React.FC<LogViewerProps> = ({
  logs: initialLogs,
  autoScroll = true,
}) => {
  const [logs, setLogs] = useState<LogEntry[]>(
    initialLogs || [
      {
        id: '1',
        timestamp: new Date(),
        level: 'info',
        message: 'EVIF server started on port 8080',
        source: 'server',
      },
      {
        id: '2',
        timestamp: new Date(Date.now() - 1000),
        level: 'debug',
        message: '正在加载插件配置',
        source: 'plugin-loader',
      },
      {
        id: '3',
        timestamp: new Date(Date.now() - 2000),
        level: 'info',
        message: 'WebSocket server initialized',
        source: 'websocket',
      },
      {
        id: '4',
        timestamp: new Date(Date.now() - 3000),
        level: 'warn',
        message: 'High memory usage detected: 75%',
        source: 'monitor',
      },
    ]
  )
  const [filter, setFilter] = useState('')
  const [levelFilter, setLevelFilter] = useState<'all' | LogEntry['level']>('all')
  const scrollRef = useRef<HTMLDivElement>(null)

  // Simulate new logs
  useEffect(() => {
    if (!autoScroll) return

    const interval = setInterval(() => {
      const newLog: LogEntry = {
        id: Date.now().toString(),
        timestamp: new Date(),
        level: ['info', 'debug', 'warn', 'error'][Math.floor(Math.random() * 4)] as LogEntry['level'],
        message: `System operation ${Math.random().toString(36).substring(7)}`,
        source: ['server', 'plugin', 'vfs', 'api'][Math.floor(Math.random() * 4)],
      }
      setLogs(prev => [newLog, ...prev].slice(0, 100))
    }, 5000)

    return () => clearInterval(interval)
  }, [autoScroll])

  const filteredLogs = logs.filter(log => {
    const matchesSearch = log.message.toLowerCase().includes(filter.toLowerCase()) ||
                         (log.source && log.source.toLowerCase().includes(filter.toLowerCase()))
    const matchesLevel = levelFilter === 'all' || log.level === levelFilter
    return matchesSearch && matchesLevel
  })

  const getLevelColor = (level: LogEntry['level']): string => {
    switch (level) {
      case 'info':
        return 'text-blue-500'
      case 'warn':
        return 'text-yellow-500'
      case 'error':
        return 'text-red-500'
      case 'debug':
        return 'text-gray-500'
    }
  }

  const getLevelBadge = (level: LogEntry['level']): 'default' | 'destructive' | 'outline' | 'secondary' => {
    switch (level) {
      case 'error':
        return 'destructive'
      case 'warn':
        return 'secondary'
      case 'info':
        return 'default'
      case 'debug':
        return 'outline'
    }
  }

  const handleExport = () => {
    const logText = logs.map(log =>
      `[${log.timestamp.toISOString()}] [${log.level.toUpperCase()}]${log.source ? ` [${log.source}]` : ''} ${log.message}`
    ).join('\n')

    const blob = new Blob([logText], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `evif-logs-${new Date().toISOString()}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  const handleClear = () => {
    setLogs([])
  }

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            <CardTitle>系统日志</CardTitle>
            <Badge variant="outline">{filteredLogs.length}</Badge>
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={handleExport}>
              <Download className="h-4 w-4" />
            </Button>
            <Button size="sm" variant="outline" onClick={handleClear}>
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Filters */}
        <div className="flex gap-2 mt-4">
          <div className="flex-1 relative">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="搜索日志..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="pl-8"
            />
          </div>
          <select
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value as any)}
            className="px-3 py-2 rounded-md border bg-background"
          >
            <option value="all">全部级别</option>
            <option value="error">错误</option>
            <option value="warn">警告</option>
            <option value="info">信息</option>
            <option value="debug">调试</option>
          </select>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden">
        <ScrollArea className="h-full" ref={scrollRef}>
          <div className="space-y-1 font-mono text-xs">
            {filteredLogs.length === 0 ? (
              <div className="text-center text-muted-foreground py-8">
                未找到日志
              </div>
            ) : (
              filteredLogs.map(log => (
                <div
                  key={log.id}
                  className="flex gap-2 py-1 px-2 hover:bg-muted/50 rounded"
                >
                  <span className="text-muted-foreground shrink-0">
                    {log.timestamp.toLocaleTimeString()}
                  </span>
                  <Badge variant={getLevelBadge(log.level)} className="shrink-0 h-5 px-1">
                    {log.level.toUpperCase()}
                  </Badge>
                  {log.source && (
                    <span className="text-muted-foreground shrink-0">
                      [{log.source}]
                    </span>
                  )}
                  <span className={getLevelColor(log.level)}>
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

export default LogViewer
