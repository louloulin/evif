import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Cpu, HardDrive, Clock, Server } from 'lucide-react'
import { SystemStatus as SystemStatusType } from '@/types/monitor'
import { Progress } from '@/components/ui/progress'
import { getMetricsStatus } from '@/services/monitor-api'

interface SystemStatusProps {
  data?: SystemStatusType
}

export const SystemStatus: React.FC<SystemStatusProps> = ({
  data: initialData,
}) => {
  const [data, setData] = useState<SystemStatusType>(
    initialData || {
      cpuUsage: 0,
      memoryUsage: 0,
      diskUsage: 0,
      uptime: 0,
    }
  )
  const [statusText, setStatusText] = useState<string>('')
  const [mountCount, setMountCount] = useState<number>(0)

  // Phase 9.2: 从后端 /api/v1/metrics/status 拉取状态与 uptime
  useEffect(() => {
    let cancelled = false
    const fetchStatus = async () => {
      try {
        const res = await getMetricsStatus()
        if (cancelled) return
        setStatusText(res.status || '')
        setMountCount(res.mounts?.count ?? 0)
        const uptimeSecs = res.uptime_secs ?? res.uptime ?? 0
        setData(prev => ({
          ...prev,
          uptime: typeof uptimeSecs === 'number' ? uptimeSecs : 0,
        }))
      } catch {
        if (!cancelled) setStatusText('离线')
      }
    }
    fetchStatus()
    const interval = setInterval(fetchStatus, 5000)
    return () => {
      cancelled = true
      clearInterval(interval)
    }
  }, [])

  const formatUptime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    const secs = seconds % 60
    return `${hours}h ${minutes}m ${secs}s`
  }

  const formatMemory = (mb: number): string => {
    const gb = mb / 1024
    return gb >= 1 ? `${gb.toFixed(2)} GB` : `${mb.toFixed(0)} MB`
  }

  const formatDisk = (mb: number): string => {
    const gb = mb / 1024
    const tb = gb / 1024
    return tb >= 1 ? `${tb.toFixed(2)} TB` : `${gb.toFixed(2)} GB`
  }

  const getStatusColor = (percentage: number): string => {
    if (percentage < 50) return 'text-green-500'
    if (percentage < 80) return 'text-yellow-500'
    return 'text-red-500'
  }

  const StatusItem: React.FC<{
    icon: React.ReactNode
    label: string
    value: string
    percentage: number
  }> = ({ icon, label, value, percentage }) => {
    const safePercentage = Math.max(0, Math.min(100, percentage))
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {icon}
            <span className="text-sm text-muted-foreground">{label}</span>
          </div>
          <span className={`text-sm font-semibold ${getStatusColor(safePercentage)}`}>
            {value}
          </span>
        </div>
        <Progress value={safePercentage} className="h-2" />
      </div>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Server className="h-5 w-5" />
          <CardTitle>系统资源</CardTitle>
          {statusText && (
            <span className="text-xs font-normal text-muted-foreground ml-2">
              {statusText === 'offline' ? '离线' : statusText} · {mountCount} 挂载
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        <StatusItem
          icon={<Cpu className="h-4 w-4" />}
          label="CPU使用率"
          value={`${data.cpuUsage.toFixed(1)}%`}
          percentage={data.cpuUsage}
        />

        <StatusItem
          icon={<HardDrive className="h-4 w-4" />}
          label="内存使用"
          value={formatMemory(data.memoryUsage)}
          percentage={(data.memoryUsage / 16384) * 100} // Assuming 16 GB max
        />

        <StatusItem
          icon={<HardDrive className="h-4 w-4" />}
          label="磁盘使用"
          value={formatDisk(data.diskUsage)}
          percentage={(data.diskUsage / (1024 * 1024)) * 100} // Assuming 1 TB max
        />

        <div className="flex items-center justify-between pt-2 border-t">
          <div className="flex items-center gap-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm text-muted-foreground">运行时间</span>
          </div>
          <span className="text-sm font-semibold">{formatUptime(data.uptime)}</span>
        </div>
      </CardContent>
    </Card>
  )
}

export default SystemStatus
