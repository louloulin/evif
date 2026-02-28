import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Activity, ArrowUp, ArrowDown } from 'lucide-react'
import { TrafficStats } from '@/types/monitor'

interface TrafficChartProps {
  data?: TrafficStats
}

export const TrafficChart: React.FC<TrafficChartProps> = ({
  data: initialData,
}) => {
  const [data, setData] = useState<TrafficStats>(
    initialData || {
      uploadSpeed: 0,
      downloadSpeed: 0,
      totalUploaded: 0,
      totalDownloaded: 0,
    }
  )

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setData(prev => ({
        uploadSpeed: Math.random() * 1024 * 1024, // 0-1 MB/s
        downloadSpeed: Math.random() * 10 * 1024 * 1024, // 0-10 MB/s
        totalUploaded: prev.totalUploaded + prev.uploadSpeed,
        totalDownloaded: prev.totalDownloaded + prev.downloadSpeed,
      }))
    }, 1000)

    return () => clearInterval(interval)
  }, [])

  const formatSpeed = (bytesPerSecond: number): string => {
    const mb = bytesPerSecond / (1024 * 1024)
    if (mb >= 1) return `${mb.toFixed(2)} MB/s`
    const kb = bytesPerSecond / 1024
    return `${kb.toFixed(2)} KB/s`
  }

  const formatTotal = (bytes: number): string => {
    const gb = bytes / (1024 * 1024 * 1024)
    if (gb >= 1) return `${gb.toFixed(2)} GB`
    const mb = bytes / (1024 * 1024)
    return `${mb.toFixed(2)} MB`
  }

  // Simple bar chart visualization
  const BarChart: React.FC<{ value: number; max: number; color: string }> = ({ value, max, color }) => {
    const percentage = max > 0 ? Math.min((value / max) * 100, 100) : 0
    return (
      <div className="space-y-1">
        <div className="h-2 bg-muted rounded-full overflow-hidden">
          <div
            className={`h-full ${color} transition-all duration-300`}
            style={{ width: `${percentage}%` }}
          />
        </div>
      </div>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            <CardTitle>网络流量</CardTitle>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Upload Speed */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm">
              <ArrowUp className="h-4 w-4 text-green-500" />
              <span className="text-muted-foreground">上传速度</span>
            </div>
            <span className="text-lg font-semibold">{formatSpeed(data.uploadSpeed)}</span>
          </div>
          <BarChart value={data.uploadSpeed} max={10 * 1024 * 1024} color="bg-green-500" />
        </div>

        {/* Download Speed */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm">
              <ArrowDown className="h-4 w-4 text-blue-500" />
              <span className="text-muted-foreground">下载速度</span>
            </div>
            <span className="text-lg font-semibold">{formatSpeed(data.downloadSpeed)}</span>
          </div>
          <BarChart value={data.downloadSpeed} max={100 * 1024 * 1024} color="bg-blue-500" />
        </div>

        {/* Totals */}
        <div className="grid grid-cols-2 gap-4 pt-4 border-t">
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">总上传</p>
            <p className="text-lg font-semibold">{formatTotal(data.totalUploaded)}</p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">总下载</p>
            <p className="text-lg font-semibold">{formatTotal(data.totalDownloaded)}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export default TrafficChart
