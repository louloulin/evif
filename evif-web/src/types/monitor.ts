export interface TrafficStats {
  uploadSpeed: number // bytes/s
  downloadSpeed: number
  totalUploaded: number // bytes
  totalDownloaded: number
}

export interface OperationStats {
  reads: number
  writes: number
  deletes: number
  mounts: number
  unmounts: number
}

export interface SystemStatus {
  cpuUsage: number // percentage
  memoryUsage: number // MB
  diskUsage: number // MB
  uptime: number // seconds
}

export interface MetricCardProps {
  title: string
  value: string | number
  unit?: string
  icon?: React.ReactNode
  trend?: {
    value: number
    isPositive: boolean
  }
}

export interface MetricData {
  id: string
  title: string
  value: number
  unit?: string
  trend?: {
    value: number
    isPositive: boolean
  }
}

export interface LogEntry {
  id: string
  timestamp: Date
  level: 'info' | 'warn' | 'error' | 'debug'
  message: string
  source?: string
}

export interface Alert {
  id: string
  severity: 'info' | 'warning' | 'error' | 'critical'
  message: string
  timestamp: Date
  resolved?: boolean
}
