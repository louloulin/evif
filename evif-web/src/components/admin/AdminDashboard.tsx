import React, { useState, useEffect } from 'react'
import { 
  Users, 
  Plug, 
  DollarSign, 
  Activity,
  TrendingUp,
  Shield,
  AlertCircle,
  RefreshCw
} from 'lucide-react'
import { toast } from '@/hooks/use-toast'
import {
  getOverview,
  getTenantStats,
  getPluginStats,
  getRevenueStats,
  getAuditLog,
  OverviewStats,
  TenantStat,
  PluginStat,
  RevenueStat,
  AuditLogEntry,
  formatCents
} from '@/services/admin-api'

// Stat Card Component
function StatCard({ 
  title, 
  value, 
  subtitle, 
  icon: Icon, 
  trend 
}: { 
  title: string
  value: string | number
  subtitle?: string
  icon: React.ElementType
  trend?: number
}) {
  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{title}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
          {subtitle && <p className="text-xs text-muted-foreground mt-1">{subtitle}</p>}
        </div>
        <div className="flex flex-col items-end gap-2">
          <div className="p-2 bg-primary/10 rounded-lg">
            <Icon className="h-5 w-5 text-primary" />
          </div>
          {trend !== undefined && (
            <div className={`flex items-center text-xs ${trend >= 0 ? 'text-green-500' : 'text-red-500'}`}>
              <TrendingUp className={`h-3 w-3 mr-1 ${trend < 0 ? 'rotate-180' : ''}`} />
              {Math.abs(trend)}%
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// Tenant Table Component
function TenantTable({ tenants }: { tenants: TenantStat[] }) {
  return (
    <div className="border rounded-lg overflow-hidden">
      <table className="w-full">
        <thead className="bg-muted/50">
          <tr>
            <th className="px-4 py-2 text-left text-sm font-medium">Name</th>
            <th className="px-4 py-2 text-left text-sm font-medium">Storage</th>
            <th className="px-4 py-2 text-left text-sm font-medium">API Calls</th>
            <th className="px-4 py-2 text-left text-sm font-medium">Plugins</th>
            <th className="px-4 py-2 text-left text-sm font-medium">Created</th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {tenants.map((tenant) => (
            <tr key={tenant.id} className="hover:bg-muted/30">
              <td className="px-4 py-2 text-sm">{tenant.name}</td>
              <td className="px-4 py-2 text-sm">
                {(tenant.storage_used / 1024 / 1024 / 1024).toFixed(2)} GB
              </td>
              <td className="px-4 py-2 text-sm">{tenant.api_calls.toLocaleString()}</td>
              <td className="px-4 py-2 text-sm">{tenant.plugins_count}</td>
              <td className="px-4 py-2 text-sm">
                {new Date(tenant.created_at).toLocaleDateString()}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

// Audit Log Component
function AuditLog({ entries }: { entries: AuditLogEntry[] }) {
  return (
    <div className="border rounded-lg overflow-hidden">
      <table className="w-full">
        <thead className="bg-muted/50">
          <tr>
            <th className="px-4 py-2 text-left text-sm font-medium">Time</th>
            <th className="px-4 py-2 text-left text-sm font-medium">Action</th>
            <th className="px-4 py-2 text-left text-sm font-medium">User</th>
            <th className="px-4 py-2 text-left text-sm font-medium">Details</th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {entries.slice(0, 10).map((entry) => (
            <tr key={entry.id} className="hover:bg-muted/30">
              <td className="px-4 py-2 text-sm">
                {new Date(entry.timestamp).toLocaleString()}
              </td>
              <td className="px-4 py-2 text-sm">
                <span className="px-2 py-0.5 bg-primary/10 text-primary rounded text-xs">
                  {entry.action}
                </span>
              </td>
              <td className="px-4 py-2 text-sm">{entry.user_id}</td>
              <td className="px-4 py-2 text-sm text-muted-foreground truncate max-w-xs">
                {entry.resource}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

// Revenue Chart Component
function RevenueChart({ data }: { data: RevenueStat[] }) {
  const maxRevenue = Math.max(...data.map(d => d.amount_cents))
  
  return (
    <div className="h-48 flex items-end gap-2">
      {data.slice(-12).map((stat, i) => (
        <div 
          key={i} 
          className="flex-1 bg-primary/20 hover:bg-primary/30 rounded-t transition-colors relative group"
          style={{ height: `${(stat.amount_cents / maxRevenue) * 100}%` }}
        >
          <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-2 py-1 bg-background border rounded text-xs opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
            {formatCents(stat.amount_cents)}
          </div>
        </div>
      ))}
    </div>
  )
}

// Main Admin Dashboard Component
export function AdminDashboard() {
  const [loading, setLoading] = useState(true)
  const [overview, setOverview] = useState<OverviewStats | null>(null)
  const [tenants, setTenants] = useState<TenantStat[]>([])
  const [plugins, setPlugins] = useState<PluginStat[]>([])
  const [revenue, setRevenue] = useState<RevenueStat[]>([])
  const [auditLog, setAuditLog] = useState<AuditLogEntry[]>([])
  const [error, setError] = useState<string | null>(null)

  const loadData = async () => {
    setLoading(true)
    setError(null)
    try {
      const [overviewData, tenantsData, pluginsData, revenueData, auditData] = await Promise.all([
        getOverview(),
        getTenantStats(),
        getPluginStats(),
        getRevenueStats({ months: 12 }),
        getAuditLog({ limit: 20 }),
      ])
      setOverview(overviewData)
      setTenants(tenantsData)
      setPlugins(pluginsData)
      setRevenue(revenueData)
      setAuditLog(auditData)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load data')
      toast({
        title: 'Error',
        description: 'Failed to load admin data',
        variant: 'destructive',
      })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadData()
  }, [])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <AlertCircle className="h-8 w-8 text-destructive" />
        <p className="text-muted-foreground">{error}</p>
        <button onClick={loadData} className="px-4 py-2 bg-primary text-primary-foreground rounded">
          Retry
        </button>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6 overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Admin Dashboard</h1>
          <p className="text-muted-foreground">System overview and management</p>
        </div>
        <button 
          onClick={loadData} 
          className="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted"
        >
          <RefreshCw className="h-4 w-4" />
          Refresh
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="Total Tenants"
          value={overview?.total_tenants ?? 0}
          subtitle={`${overview?.active_tenants ?? 0} active`}
          icon={Users}
          trend={overview?.growth_rate}
        />
        <StatCard
          title="Total Plugins"
          value={overview?.total_plugins ?? 0}
          subtitle="installed"
          icon={Plug}
        />
        <StatCard
          title="Total Revenue"
          value={formatCents(overview?.total_revenue_cents ?? 0)}
          subtitle="all time"
          icon={DollarSign}
        />
        <StatCard
          title="API Calls"
          value={(overview?.total_api_calls ?? 0).toLocaleString()}
          subtitle="this month"
          icon={Activity}
        />
      </div>

      {/* Revenue Chart */}
      <div className="bg-card rounded-lg border p-4">
        <h2 className="text-lg font-semibold mb-4">Revenue (Last 12 Months)</h2>
        <RevenueChart data={revenue} />
      </div>

      {/* Two Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Top Plugins */}
        <div className="bg-card rounded-lg border p-4">
          <h2 className="text-lg font-semibold mb-4">Top Plugins</h2>
          <div className="space-y-3">
            {plugins.slice(0, 5).map((plugin) => (
              <div key={plugin.id} className="flex items-center justify-between">
                <div>
                  <p className="font-medium">{plugin.name}</p>
                  <p className="text-sm text-muted-foreground">by {plugin.author}</p>
                </div>
                <div className="text-right">
                  <p className="font-medium">{formatCents(plugin.revenue_cents)}</p>
                  <p className="text-sm text-muted-foreground">{plugin.downloads} downloads</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Recent Activity */}
        <div className="bg-card rounded-lg border p-4">
          <h2 className="text-lg font-semibold mb-4">Recent Activity</h2>
          <AuditLog entries={auditLog} />
        </div>
      </div>

      {/* Tenant Table */}
      <div className="bg-card rounded-lg border p-4">
        <h2 className="text-lg font-semibold mb-4">Tenant Management</h2>
        <TenantTable tenants={tenants} />
      </div>
    </div>
  )
}

export default AdminDashboard
