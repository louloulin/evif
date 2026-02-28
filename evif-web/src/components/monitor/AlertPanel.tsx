import React, { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Bell, AlertCircle, AlertTriangle, CheckCircle, X } from 'lucide-react'
import { Alert as AlertType } from '@/types/monitor'
import { ScrollArea } from '@/components/ui/scroll-area'

interface AlertPanelProps {
  alerts?: AlertType[]
}

export const AlertPanel: React.FC<AlertPanelProps> = ({
  alerts: initialAlerts,
}) => {
  const [alerts, setAlerts] = useState<AlertType[]>(
    initialAlerts || [
      {
        id: '1',
        severity: 'info',
        message: 'System started successfully',
        timestamp: new Date(),
        resolved: true,
      },
      {
        id: '2',
        severity: 'warning',
        message: 'High memory usage detected (75%)',
        timestamp: new Date(Date.now() - 60000),
        resolved: false,
      },
    ]
  )

  const handleDismiss = (id: string) => {
    setAlerts(prev => prev.filter(alert => alert.id !== id))
  }

  const getAlertIcon = (severity: AlertType['severity']) => {
    switch (severity) {
      case 'critical':
        return <AlertCircle className="h-4 w-4" />
      case 'error':
        return <X className="h-4 w-4" />
      case 'warning':
        return <AlertTriangle className="h-4 w-4" />
      case 'info':
        return <CheckCircle className="h-4 w-4" />
    }
  }

  const getAlertColor = (severity: AlertType['severity']): string => {
    switch (severity) {
      case 'critical':
        return 'border-red-500 bg-red-500/10'
      case 'error':
        return 'border-red-500 bg-red-500/10'
      case 'warning':
        return 'border-yellow-500 bg-yellow-500/10'
      case 'info':
        return 'border-blue-500 bg-blue-500/10'
    }
  }

  const getBadgeVariant = (severity: AlertType['severity']): 'default' | 'destructive' | 'outline' | 'secondary' => {
    switch (severity) {
      case 'critical':
      case 'error':
        return 'destructive'
      case 'warning':
        return 'secondary'
      case 'info':
        return 'default'
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Bell className="h-5 w-5" />
            <CardTitle>Alerts</CardTitle>
            <Badge variant="outline">{alerts.length}</Badge>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-64">
          <div className="space-y-2">
            {alerts.length === 0 ? (
              <div className="text-center text-muted-foreground py-8">
                No alerts
              </div>
            ) : (
              alerts.map(alert => (
                <div
                  key={alert.id}
                  className={`flex items-start gap-4 p-3 rounded-lg border ${getAlertColor(alert.severity)} ${
                    alert.resolved ? 'opacity-50' : ''
                  }`}
                >
                  <div className="mt-0.5">
                    {getAlertIcon(alert.severity)}
                  </div>
                  <div className="flex-1 space-y-1">
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-medium">{alert.message}</p>
                      <Badge variant={getBadgeVariant(alert.severity)} className="text-xs">
                        {alert.severity}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {alert.timestamp.toLocaleString()}
                    </p>
                  </div>
                  {!alert.resolved && (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-8 w-8 p-0"
                      onClick={() => handleDismiss(alert.id)}
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  )
}

export default AlertPanel
