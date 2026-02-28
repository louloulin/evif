/**
 * Monitor Dashboard View
 * Displays system metrics, logs, and alerts
 */

import React, { useState, useEffect } from 'react';
import { SystemStatus } from './monitor/SystemStatus';
import { MetricCard } from './monitor/MetricCard';
import { TrafficChart } from './monitor/TrafficChart';
import { OperationChart } from './monitor/OperationChart';
import { LogViewer } from './monitor/LogViewer';
import { AlertPanel } from './monitor/AlertPanel';
import { ErrorBoundary } from './ErrorBoundary';
import { getMetricsStatus, getMetricsTraffic, getMetricsOperations } from '@/services/monitor-api';
import type { MetricData, LogEntry, Alert } from '@/types/monitor';

export const MonitorView: React.FC = () => {
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'logs' | 'alerts'>('overview');

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const [statusData, trafficData, operationsData] = await Promise.all([
          getMetricsStatus().catch(() => null),
          getMetricsTraffic().catch(() => null),
          getMetricsOperations().catch(() => []),
        ]);

        // Convert API data to MetricData format
        const metrics: MetricData[] = [];

        if (statusData?.traffic && typeof statusData.traffic === 'object') {
          const traffic = statusData.traffic;
          const safeTrend = (val: number | undefined, isPositive: boolean) => {
            const safeVal = typeof val === 'number' && !isNaN(val) ? val : 0;
            return { value: safeVal, isPositive };
          };
          metrics.push(
            {
              id: 'requests',
              title: '总请求数',
              value: typeof traffic.total_requests === 'number' ? traffic.total_requests : 0,
              unit: 'count',
              trend: safeTrend(5, true)
            },
            {
              id: 'bytes-read',
              title: '读取字节数',
              value: typeof traffic.total_bytes_read === 'number' ? traffic.total_bytes_read : 0,
              unit: 'bytes',
              trend: safeTrend(12, true)
            },
            {
              id: 'bytes-written',
              title: '写入字节数',
              value: typeof traffic.total_bytes_written === 'number' ? traffic.total_bytes_written : 0,
              unit: 'bytes',
              trend: safeTrend(8, true)
            },
            {
              id: 'errors',
              title: '错误',
              value: typeof traffic.total_errors === 'number' ? traffic.total_errors : 0,
              unit: 'count',
              trend: safeTrend(2, false)
            }
          );
        }

        setMetrics(metrics);
        setLogs([]); // Logs would come from a separate API
        setAlerts([]); // Alerts would come from a separate API
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load monitoring data');
      } finally {
        setLoading(false);
      }
    };

    fetchData();

    // Refresh every 5 seconds
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
          <p className="text-sm text-muted-foreground">加载监控数据中...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-lg text-destructive mb-2">Error loading monitoring data</p>
          <p className="text-sm text-muted-foreground">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Tab Navigation */}
      <div className="flex items-center gap-2 px-4 py-2 md:px-6 md:py-3 border-b bg-muted/30">
        <button
          className={`px-3 py-1 md:px-4 md:py-2 rounded text-sm md:text-base transition-colors min-h-[44px] min-w-[44px] active:scale-95 ${
            activeTab === 'overview'
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-muted'
          }`}
          onClick={() => setActiveTab('overview')}
        >
          概览
        </button>
        <button
          className={`px-3 py-1 md:px-4 md:py-2 rounded text-sm md:text-base transition-colors min-h-[44px] min-w-[44px] active:scale-95 ${
            activeTab === 'logs'
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-muted'
          }`}
          onClick={() => setActiveTab('logs')}
        >
          日志
        </button>
        <button
          className={`px-3 py-1 md:px-4 md:py-2 rounded text-sm md:text-base transition-colors min-h-[44px] min-w-[44px] active:scale-95 ${
            activeTab === 'alerts'
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-muted'
          }`}
          onClick={() => setActiveTab('alerts')}
        >
          告警 {alerts.length > 0 && `(${alerts.length})`}
        </button>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'overview' && (
          <div className="p-4 md:p-6 lg:p-8 space-y-4 md:space-y-6">
            {/* System Status */}
            <ErrorBoundary>
              <SystemStatus />
            </ErrorBoundary>

            {/* Metric Cards - 响应式网格: 移动1列 → 平板2列 → 桌面4列 */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 md:gap-4 lg:gap-6">
              {metrics.map((metric) => (
                <ErrorBoundary key={metric.id}>
                  <MetricCard
                    title={metric.title}
                    value={metric.value}
                    unit={metric.unit}
                    trend={metric.trend}
                  />
                </ErrorBoundary>
              ))}
            </div>

            {/* Charts - 响应式: 移动全宽 → 桌面双列 */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6">
              <ErrorBoundary>
                <TrafficChart />
              </ErrorBoundary>
              <ErrorBoundary>
                <OperationChart />
              </ErrorBoundary>
            </div>
          </div>
        )}

        {activeTab === 'logs' && (
          <div className="h-full">
            <ErrorBoundary>
              <LogViewer logs={logs} />
            </ErrorBoundary>
          </div>
        )}

        {activeTab === 'alerts' && (
          <div className="p-4 md:p-6 lg:p-8">
            <ErrorBoundary>
              <AlertPanel alerts={alerts} />
            </ErrorBoundary>
          </div>
        )}
      </div>
    </div>
  );
};
