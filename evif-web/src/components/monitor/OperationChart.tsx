import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { BarChart3, FileText, Save, Trash2, FolderOpen, Folder } from 'lucide-react'
import { OperationStats } from '@/types/monitor'

interface OperationChartProps {
  data?: OperationStats
}

export const OperationChart: React.FC<OperationChartProps> = ({
  data: initialData,
}) => {
  const [data, setData] = useState<OperationStats>(
    initialData || {
      reads: 0,
      writes: 0,
      deletes: 0,
      mounts: 0,
      unmounts: 0,
    }
  )

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setData(prev => ({
        reads: prev.reads + Math.floor(Math.random() * 5),
        writes: prev.writes + Math.floor(Math.random() * 3),
        deletes: prev.deletes + Math.floor(Math.random() * 2),
        mounts: prev.mounts,
        unmounts: prev.unmounts,
      }))
    }, 2000)

    return () => clearInterval(interval)
  }, [])

  const operations = [
    {
      name: '读取',
      value: data.reads,
      icon: <FileText className="h-4 w-4 text-blue-500" />,
      color: 'bg-blue-500',
    },
    {
      name: '写入',
      value: data.writes,
      icon: <Save className="h-4 w-4 text-green-500" />,
      color: 'bg-green-500',
    },
    {
      name: '删除',
      value: data.deletes,
      icon: <Trash2 className="h-4 w-4 text-red-500" />,
      color: 'bg-red-500',
    },
    {
      name: '挂载',
      value: data.mounts,
      icon: <FolderOpen className="h-4 w-4 text-purple-500" />,
      color: 'bg-purple-500',
    },
    {
      name: '卸载',
      value: data.unmounts,
      icon: <Folder className="h-4 w-4 text-yellow-500" />,
      color: 'bg-yellow-500',
    },
  ]

  const maxValue = Math.max(...operations.map(op => op.value), 1)

  const StatBar: React.FC<{
    name: string
    value: number
    icon: React.ReactNode
    color: string
    max: number
  }> = ({ name, value, icon, color, max }) => {
    const percentage = max > 0 ? Math.min((value / max) * 100, 100) : 0
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {icon}
            <span className="text-sm text-muted-foreground">{name}</span>
          </div>
          <span className="text-sm font-semibold">{value.toLocaleString()}</span>
        </div>
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
        <div className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5" />
          <CardTitle>操作</CardTitle>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {operations.map(op => (
          <StatBar
            key={op.name}
            name={op.name}
            value={op.value}
            icon={op.icon}
            color={op.color}
            max={maxValue}
          />
        ))}
      </CardContent>
    </Card>
  )
}

export default OperationChart
