import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Activity, Cpu, HardDrive, Clock, CheckCircle, XCircle, AlertCircle } from 'lucide-react'

interface PluginStats {
  uptime: string
  memory: number
  operations: number
  errors: number
  lastActivity: string
}

interface PluginStatusProps {
  pluginId: string
  pluginName: string
}

export const PluginStatus: React.FC<PluginStatusProps> = ({
  pluginId,
  pluginName,
}) => {
  const [stats, setStats] = useState<PluginStats>({
    uptime: '0m 0s',
    memory: 0,
    operations: 0,
    errors: 0,
    lastActivity: '从未',
  })
  const [health, setHealth] = useState<'healthy' | 'warning' | 'error'>('healthy')

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setStats(prev => ({
        ...prev,
        operations: prev.operations + Math.floor(Math.random() * 5),
        memory: Math.min(100, Math.max(0, prev.memory + (Math.random() - 0.5) * 10)),
      }))
    }, 2000)

    return () => clearInterval(interval)
  }, [pluginId])

  const getHealthIcon = () => {
    switch (health) {
      case 'healthy':
        return <CheckCircle className="h-4 w-4 text-green-500" />
      case 'warning':
        return <AlertCircle className="h-4 w-4 text-yellow-500" />
      case 'error':
        return <XCircle className="h-4 w-4 text-red-500" />
    }
  }

  const getHealthBadge = () => {
    switch (health) {
      case 'healthy':
        return <Badge className="bg-green-500">健康</Badge>
      case 'warning':
        return <Badge variant="secondary" className="bg-yellow-500 text-white">警告</Badge>
      case 'error':
        return <Badge variant="destructive">错误</Badge>
    }
  }

  const StatCard: React.FC<{
    icon: React.ReactNode
    label: string
    value: string | number
    unit?: string
  }> = ({ icon, label, value, unit }) => (
    <div className="flex items-center gap-4 p-3 border rounded-lg">
      <div className="text-muted-foreground">{icon}</div>
      <div className="flex-1">
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="text-lg font-semibold">
          {value}
          {unit && <span className="text-sm font-normal text-muted-foreground"> {unit}</span>}
        </p>
      </div>
    </div>
  )

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            <CardTitle className="text-lg">插件状态</CardTitle>
            <Badge variant="outline">{pluginName}</Badge>
          </div>
          <div className="flex items-center gap-2">
            {getHealthIcon()}
            {getHealthBadge()}
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <StatCard
            icon={<Clock className="h-4 w-4" />}
            label="运行时间"
            value={stats.uptime}
          />
          <StatCard
            icon={<Cpu className="h-4 w-4" />}
            label="内存"
            value={stats.memory.toFixed(1)}
            unit="MB"
          />
          <StatCard
            icon={<HardDrive className="h-4 w-4" />}
            label="操作数"
            value={stats.operations.toLocaleString()}
          />
          <StatCard
            icon={<AlertCircle className="h-4 w-4" />}
            label="错误"
            value={stats.errors}
          />
        </div>

        <div className="pt-2 border-t">
          <p className="text-xs text-muted-foreground">
            最后活动: {stats.lastActivity}
          </p>
        </div>
      </CardContent>
    </Card>
  )
}

export default PluginStatus
