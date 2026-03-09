/**
 * FUSE Status Panel Component
 * Displays FUSE mount status and mount points
 */

import React, { useState, useEffect } from 'react';
import { listMounts, type MountInfo } from '@/services/monitor-api';

export const FUSEStatusPanel: React.FC = () => {
  const [mounts, setMounts] = useState<MountInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMounts = async () => {
      try {
        const data = await listMounts();
        setMounts(data.mounts || []);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch mounts');
      } finally {
        setLoading(false);
      }
    };

    fetchMounts();

    // Refresh every 10 seconds
    const interval = setInterval(fetchMounts, 10000);
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
              listMounts()
                .then((data) => {
                  setMounts(data.mounts || []);
                  setError(null);
                })
                .catch((err) => {
                  setError(err instanceof Error ? err.message : 'Failed to fetch mounts');
                })
                .finally(() => setLoading(false));
            }}
          >
            刷新
          </button>
        </div>
      )}
    </div>
  );
};
