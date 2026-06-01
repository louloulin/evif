import React, { useState, useEffect } from 'react'
import { 
  CreditCard, 
  TrendingUp, 
  HardDrive, 
  Zap,
  AlertTriangle,
  Check,
  ExternalLink,
  RefreshCw,
  Loader2
} from 'lucide-react'
import { toast } from '@/hooks/use-toast'
import {
  getUsage,
  getUsageHistory,
  getSubscription,
  getInvoices,
  createCheckout,
  createPortal,
  UsageResponse,
  Subscription,
  Invoice,
  UsageHistoryEntry,
  PricingPlan,
  formatBytes,
  formatCents,
  getUsagePercent,
} from '@/services/billing-api'

// Pricing Plans Configuration
const PRICING_PLANS = {
  free: { name: 'Free', price: 0, features: ['1,000 API calls/day', '100MB storage', '1 agent'] },
  pro: { name: 'Pro', price: 2900, features: ['100,000 API calls/day', '10GB storage', '10 agents'] },
  team: { name: 'Team', price: 9900, features: ['1,000,000 API calls/day', '100GB storage', '50 agents'] },
  enterprise: { name: 'Enterprise', price: 49900, features: ['Unlimited', 'Unlimited storage', 'Unlimited agents'] },
}

// Quota Bar Component
function QuotaBar({ label, used, limit, unit }: { label: string; used: number; limit: number; unit: string }) {
  const percent = getUsagePercent({ used, limit, unit })
  const isWarning = percent >= 80
  const isDanger = percent >= 100
  
  return (
    <div className="space-y-2">
      <div className="flex justify-between text-sm">
        <span>{label}</span>
        <span className={isDanger ? 'text-red-500' : isWarning ? 'text-yellow-500' : ''}>
          {formatBytes(used)} / {formatBytes(limit)}
        </span>
      </div>
      <div className="h-2 bg-muted rounded-full overflow-hidden">
        <div 
          className={`h-full transition-all ${isDanger ? 'bg-red-500' : isWarning ? 'bg-yellow-500' : 'bg-primary'}`}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
    </div>
  )
}

// Plan Card Component
function PlanCard({ 
  plan, 
  current, 
  onSelect 
}: { 
  plan: typeof PRICING_PLANS.free
  current: boolean
  onSelect: () => void 
}) {
  return (
    <div className={`border rounded-lg p-4 ${current ? 'border-primary ring-2 ring-primary/20' : ''}`}>
      <div className="flex justify-between items-start mb-4">
        <h3 className="font-semibold">{plan.name}</h3>
        <div className="text-right">
          <p className="text-xl font-bold">{plan.price === 0 ? 'Free' : formatCents(plan.price)}</p>
          {plan.price > 0 && <p className="text-xs text-muted-foreground">/month</p>}
        </div>
      </div>
      <ul className="space-y-2 mb-4">
        {plan.features.map((feature, i) => (
          <li key={i} className="flex items-center gap-2 text-sm text-muted-foreground">
            <Check className="h-4 w-4 text-green-500" />
            {feature}
          </li>
        ))}
      </ul>
      {current ? (
        <button disabled className="w-full py-2 border rounded-lg bg-muted text-muted-foreground">
          Current Plan
        </button>
      ) : (
        <button 
          onClick={onSelect}
          className="w-full py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
        >
          {plan.price > 0 ? 'Upgrade' : 'Downgrade'}
        </button>
      )}
    </div>
  )
}

// Invoice Row Component
function InvoiceRow({ invoice }: { invoice: Invoice }) {
  return (
    <tr className="border-b">
      <td className="px-4 py-3">{invoice.id}</td>
      <td className="px-4 py-3">
        {new Date(invoice.period.start).toLocaleDateString()} - {new Date(invoice.period.end).toLocaleDateString()}
      </td>
      <td className="px-4 py-3">{formatCents(invoice.amount_cents)}</td>
      <td className="px-4 py-3">
        <span className={`px-2 py-1 rounded text-xs ${
          invoice.status === 'paid' ? 'bg-green-100 text-green-800' :
          invoice.status === 'unpaid' ? 'bg-yellow-100 text-yellow-800' :
          'bg-gray-100 text-gray-800'
        }`}>
          {invoice.status}
        </span>
      </td>
      <td className="px-4 py-3">
        {invoice.pdf_url && (
          <a href={invoice.pdf_url} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline flex items-center gap-1">
            PDF <ExternalLink className="h-3 w-3" />
          </a>
        )}
      </td>
    </tr>
  )
}

// Main Billing Dashboard Component
export function BillingDashboard() {
  const [loading, setLoading] = useState(true)
  const [upgrading, setUpgrading] = useState(false)
  const [usage, setUsage] = useState<UsageResponse | null>(null)
  const [subscription, setSubscription] = useState<Subscription | null>(null)
  const [invoices, setInvoices] = useState<Invoice[]>([])
  const [history, setHistory] = useState<UsageHistoryEntry[]>([])
  const [error, setError] = useState<string | null>(null)

  const loadData = async () => {
    setLoading(true)
    setError(null)
    try {
      const [usageData, subData, invoiceData, historyData] = await Promise.all([
        getUsage(),
        getSubscription(),
        getInvoices(),
        getUsageHistory(6),
      ])
      setUsage(usageData)
      setSubscription(subData)
      setInvoices(invoiceData)
      setHistory(historyData)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load billing data')
    } finally {
      setLoading(false)
    }
  }

  const handlePlanChange = async (plan: PricingPlan) => {
    setUpgrading(true)
    try {
      const session = await createCheckout({
        plan,
        success_url: window.location.href + '?success=true',
        cancel_url: window.location.href + '?cancelled=true',
      })
      // Redirect to Stripe Checkout
      window.location.href = session.url
    } catch (err) {
      toast({
        title: 'Error',
        description: 'Failed to create checkout session',
        variant: 'destructive',
      })
    } finally {
      setUpgrading(false)
    }
  }

  const handleManageSubscription = async () => {
    try {
      const session = await createPortal({ return_url: window.location.href })
      window.location.href = session.url
    } catch (err) {
      toast({
        title: 'Error',
        description: 'Failed to open billing portal',
        variant: 'destructive',
      })
    }
  }

  useEffect(() => {
    loadData()
  }, [])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <AlertTriangle className="h-8 w-8 text-destructive" />
        <p className="text-muted-foreground">{error}</p>
        <button onClick={loadData} className="px-4 py-2 bg-primary text-primary-foreground rounded">
          Retry
        </button>
      </div>
    )
  }

  const currentPlan = subscription?.plan ?? 'free'

  return (
    <div className="p-6 space-y-6 overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Billing Dashboard</h1>
          <p className="text-muted-foreground">Manage your subscription and usage</p>
        </div>
        <button 
          onClick={loadData} 
          className="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted"
        >
          <RefreshCw className="h-4 w-4" />
          Refresh
        </button>
      </div>

      {/* Current Plan & Usage */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Current Plan */}
        <div className="lg:col-span-2 bg-card rounded-lg border p-6">
          <h2 className="text-lg font-semibold mb-4">Current Subscription</h2>
          <div className="flex items-center justify-between mb-6">
            <div>
              <p className="text-3xl font-bold">{PRICING_PLANS[currentPlan].name}</p>
              <p className="text-muted-foreground">
                {subscription?.status === 'active' ? 'Active' : subscription?.status}
              </p>
            </div>
            <button 
              onClick={handleManageSubscription}
              className="px-4 py-2 border rounded-lg hover:bg-muted"
            >
              Manage Subscription
            </button>
          </div>
          
          {/* Usage Quotas */}
          <div className="space-y-4">
            <QuotaBar 
              label="API Calls" 
              used={usage?.quotas.api_calls.used ?? 0}
              limit={usage?.quotas.api_calls.limit ?? 0}
              unit="calls"
            />
            <QuotaBar 
              label="Storage" 
              used={usage?.quotas.storage.used ?? 0}
              limit={usage?.quotas.storage.limit ?? 0}
              unit="bytes"
            />
            <QuotaBar 
              label="Agents" 
              used={usage?.quotas.agents.used ?? 0}
              limit={usage?.quotas.agents.limit ?? 0}
              unit="agents"
            />
          </div>
        </div>

        {/* Quick Stats */}
        <div className="space-y-4">
          <div className="bg-card rounded-lg border p-4">
            <div className="flex items-center gap-3 mb-2">
              <CreditCard className="h-5 w-5 text-primary" />
              <span className="font-medium">Next Billing</span>
            </div>
            <p className="text-2xl font-bold">
              {subscription?.next_billing_at 
                ? new Date(subscription.next_billing_at).toLocaleDateString()
                : 'N/A'
              }
            </p>
          </div>
          <div className="bg-card rounded-lg border p-4">
            <div className="flex items-center gap-3 mb-2">
              <Zap className="h-5 w-5 text-primary" />
              <span className="font-medium">This Month</span>
            </div>
            <p className="text-2xl font-bold">
              {(usage?.quotas.api_calls.used ?? 0).toLocaleString()}
            </p>
            <p className="text-sm text-muted-foreground">API calls</p>
          </div>
        </div>
      </div>

      {/* Usage History Chart */}
      <div className="bg-card rounded-lg border p-6">
        <h2 className="text-lg font-semibold mb-4">Usage History</h2>
        <div className="h-48 flex items-end gap-2">
          {history.map((entry, i) => (
            <div 
              key={i} 
              className="flex-1 bg-primary/20 hover:bg-primary/30 rounded-t transition-colors cursor-pointer"
              style={{ height: `${Math.min((entry.api_calls / Math.max(...history.map(h => h.api_calls))) * 100, 100)}%` }}
              title={`${entry.date}: ${entry.api_calls.toLocaleString()} calls`}
            />
          ))}
        </div>
        <div className="flex justify-between mt-2 text-xs text-muted-foreground">
          {history.slice(-6).map((entry, i) => (
            <span key={i}>{new Date(entry.date).toLocaleDateString('en', { month: 'short' })}</span>
          ))}
        </div>
      </div>

      {/* Pricing Plans */}
      <div className="bg-card rounded-lg border p-6">
        <h2 className="text-lg font-semibold mb-4">Available Plans</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {(Object.entries(PRICING_PLANS) as [PricingPlan, typeof PRICING_PLANS.free][]).map(([key, plan]) => (
            <PlanCard 
              key={key}
              plan={plan}
              current={currentPlan === key}
              onSelect={() => handlePlanChange(key)}
            />
          ))}
        </div>
      </div>

      {/* Invoices */}
      <div className="bg-card rounded-lg border p-6">
        <h2 className="text-lg font-semibold mb-4">Invoice History</h2>
        <table className="w-full">
          <thead>
            <tr className="text-left text-sm text-muted-foreground border-b">
              <th className="px-4 py-2">Invoice ID</th>
              <th className="px-4 py-2">Period</th>
              <th className="px-4 py-2">Amount</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2">Download</th>
            </tr>
          </thead>
          <tbody>
            {invoices.map((invoice) => (
              <InvoiceRow key={invoice.id} invoice={invoice} />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default BillingDashboard
