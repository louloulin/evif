/**
 * FUSE Status Panel Component
 * Displays FUSE mount status, mount points, and operation logs
 */

import React, { useState, useEffect } from 'react';
import { listMounts, getMetricsOperations, type MountInfo, type OperationStats } from '@/services/monitor-api';

interface OperationLog {
  id: string;
  type: 'read' | 'write' | 'list' | 'delete' | 'other';
  count: number;
  bytes: number;
  errors: number;
  timestamp: string;
}

export const FUSEStatusPanel: React.FC = () => {
  const [mounts, setMounts] = useState<MountInfo[]>([]);
  const [operations, setOperations] = useState<OperationLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [mountsData, opsData] = await Promise.all([
          listMounts(),
          getMetricsOperations().catch(() => [])
        ]);

        setMounts(mountsData.mounts || []);

        // Convert operation stats to operation logs
        const logs: OperationLog[] = (opsData as OperationStats[]).map((op, index) => ({
          id: `op-${index}-${Date.now()}`,
          type: op.operation as OperationLog['type'],
          count: op.count,
          bytes: op.bytes,
          errors: op.errors,
          timestamp: new Date().toISOString()
        }));

        setOperations(logs);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch data');
      } finally {
        setLoading(false);
      }
    };

    fetchData();

    // Refresh every 10 seconds
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="bg-card rounded-lg border p-4">
        <div className="flex items-center justify-center py-4">
          <div className="inline-block animate-spin rounded-full h-5 w-5 border-b-2 border-primary mr-2"></div>
          <span className="text-sm text-muted-foreground">加载挂载信息...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card rounded-lg border">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <div className="flex items-center gap-2">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-primary"
          >
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
            <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
            <line x1="12" y1="22.08" x2="12" y2="12" />
          </svg>
          <h3 className="font-semibold text-sm">FUSE 挂载状态</h3>
        </div>
        <div className="flex items-center gap-2">
          <span
            className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${
              mounts.length > 0
                ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400'
            }`}
          >
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                mounts.length > 0 ? 'bg-green-500' : 'bg-yellow-500'
              }`}
            ></span>
            {mounts.length > 0 ? '已挂载' : '未挂载'}
          </span>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <div className="px-4 py-3 bg-destructive/10 text-destructive text-sm">
          加载失败: {error}
        </div>
      )}

      {/* Mount List */}
      <div className="p-4">
        {mounts.length === 0 ? (
          <div className="text-center py-6 text-muted-foreground">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="40"
              height="40"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="mx-auto mb-2 opacity-50"
            >
              <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
            </svg>
            <p className="text-sm">暂无挂载点</p>
            <p className="text-xs mt-1">使用 /api/v1/mount 接口挂载插件</p>
          </div>
        ) : (
          <div className="space-y-2">
            <div className="text-xs text-muted-foreground mb-3">
              共 {mounts.length} 个挂载点
            </div>
            {mounts.map((mount, index) => (
              <div
                key={index}
                className="flex items-center justify-between p-3 rounded-lg bg-muted/50 hover:bg-muted transition-colors"
              >
                <div className="flex items-center gap-3">
                  <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-primary/10">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className="text-primary"
                    >
                      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                    </svg>
                  </div>
                  <div>
                    <p className="font-mono text-sm font-medium">{mount.path}</p>
                    <p className="text-xs text-muted-foreground">插件: {mount.plugin}</p>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <span className="w-2 h-2 rounded-full bg-green-500"></span>
                  <span className="text-xs text-green-600 dark:text-green-400">活跃</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer Actions */}
      {mounts.length > 0 && (
        <div className="flex items-center gap-2 px-4 py-3 border-t bg-muted/30">
          <button
            className="flex-1 px-3 py-1.5 text-xs font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            onClick={() => {
              // Refresh mounts
              setLoading(true);
              Promise.all([listMounts(), getMetricsOperations()])
                .then(([mountsData, opsData]) => {
                  setMounts(mountsData.mounts || []);
                  const logs: OperationLog[] = ((opsData as OperationStats[]) || []).map((op, index) => ({
                    id: `op-${index}-${Date.now()}`,
                    type: op.operation as OperationLog['type'],
                    count: op.count,
                    bytes: op.bytes,
                    errors: op.errors,
                    timestamp: new Date().toISOString()
                  }));
                  setOperations(logs);
                  setError(null);
                })
                .catch((err) => {
                  setError(err instanceof Error ? err.message : 'Failed to fetch data');
                })
                .finally(() => setLoading(false));
            }}
          >
            刷新
          </button>
          <button
            className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
              showLogs
                ? 'bg-secondary text-secondary-foreground'
                : 'bg-muted text-muted-foreground hover:bg-muted/80'
            }`}
            onClick={() => setShowLogs(!showLogs)}
          >
            {showLogs ? '隐藏日志' : '操作日志'}
          </button>
        </div>
      )}

      {/* Operation Logs */}
      {showLogs && (
        <div className="border-t">
          <div className="flex items-center justify-between px-4 py-2 bg-muted/50">
            <h4 className="text-sm font-medium">操作日志</h4>
            <span className="text-xs text-muted-foreground">{operations.length} 条记录</span>
          </div>
          <div className="max-h-64 overflow-auto">
            {operations.length === 0 ? (
              <div className="px-4 py-6 text-center text-muted-foreground">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="mx-auto mb-2 opacity-50"
                >
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                  <polyline points="14 2 14 8 20 8" />
                  <line x1="16" y1="13" x2="8" y2="13" />
                  <line x1="16" y1="17" x2="8" y2="17" />
                  <polyline points="10 9 9 9 8 9" />
                </svg>
                <p className="text-xs">暂无操作记录</p>
              </div>
            ) : (
              <div className="divide-y">
                {operations.map((op) => (
                  <div key={op.id} className="px-4 py-2 hover:bg-muted/30 transition-colors">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span
                          className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                            op.type === 'read'
                              ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400'
                              : op.type === 'write'
                              ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                              : op.type === 'delete'
                              ? 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400'
                              : 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400'
                          }`}
                        >
                          {op.type === 'read' && '📖'}
                          {op.type === 'write' && '📝'}
                          {op.type === 'delete' && '🗑️'}
                          {op.type === 'list' && '📋'}
                          {op.type === 'other' && '⚙️'}
                          {op.type.toUpperCase()}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {op.count} 次操作
                        </span>
                      </div>
                      <div className="flex items-center gap-3 text-xs">
                        <span className="text-muted-foreground">
                          {formatBytes(op.bytes)}
                        </span>
                        {op.errors > 0 && (
                          <span className="text-destructive">
                            {op.errors} 错误
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

// Helper function to format bytes
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};
