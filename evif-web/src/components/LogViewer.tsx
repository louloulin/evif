import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useWebSocket, type WebSocketStatus } from '@/hooks/useWebSocket';

export interface WsLogEntry {
  id: string;
  timestamp: Date;
  level: 'info' | 'warn' | 'error' | 'debug';
  source?: string;
  message: string;
}

type LevelFilter = 'ALL' | 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';

const MAX_LOGS = 1000;

interface LogViewerProps {
  /** WebSocket URL for the log stream. If omitted, mock mode is used. */
  wsUrl?: string;
  /** Whether to start in mock/demo mode (simulates logs without backend). Defaults to true when no url provided. */
  mockMode?: boolean;
}

const IconLogs = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
    <line x1="8" y1="21" x2="16" y2="21" />
    <line x1="12" y1="17" x2="12" y2="21" />
    <line x1="6" y1="8" x2="14" y2="8" />
    <line x1="6" y1="11" x2="10" y2="11" />
  </svg>
);

const StatusDot: React.FC<{ status: WebSocketStatus }> = ({ status }) => {
  const colors: Record<WebSocketStatus, string> = {
    connected: '#22c55e',
    connecting: '#eab308',
    reconnecting: '#eab308',
    disconnected: '#ef4444',
  };
  const labels: Record<WebSocketStatus, string> = {
    connected: 'Connected',
    connecting: 'Connecting...',
    reconnecting: 'Reconnecting...',
    disconnected: 'Disconnected',
  };
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, fontSize: 11, color: colors[status] }}>
      <span style={{ width: 7, height: 7, borderRadius: '50%', backgroundColor: colors[status], display: 'inline-block' }} />
      {labels[status]}
    </span>
  );
};

const LogViewer: React.FC<LogViewerProps> = ({ wsUrl, mockMode: propMockMode }) => {
  const [logs, setLogs] = useState<WsLogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<LevelFilter>('ALL');
  const [searchText, setSearchText] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const userScrolledUpRef = useRef(false);

  const effectiveMockMode = propMockMode ?? !wsUrl;

  const { status, lastMessage } = useWebSocket({
    url: wsUrl,
    mockMode: effectiveMockMode,
    onMessage: useCallback((data: unknown) => {
      const msg = data as Record<string, unknown>;
      const entry: WsLogEntry = {
        id: String(msg.id ?? Date.now()),
        timestamp: msg.timestamp ? new Date(msg.timestamp as string) : new Date(),
        level: (msg.level as WsLogEntry['level']) ?? 'info',
        source: msg.source as string | undefined,
        message: String(msg.message ?? msg.text ?? ''),
      };
      setLogs(prev => {
        const next = [...prev, entry];
        return next.length > MAX_LOGS ? next.slice(next.length - MAX_LOGS) : next;
      });
    }, []),
  });

  // Auto-scroll to bottom
  useEffect(() => {
    if (!autoScroll || userScrolledUpRef.current) return;
    const el = scrollContainerRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [logs, autoScroll]);

  const handleScroll = useCallback(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    userScrolledUpRef.current = !isAtBottom;
    if (isAtBottom) {
      setAutoScroll(true);
    }
  }, []);

  const filteredLogs = useMemo(() => {
    return logs.filter(log => {
      if (levelFilter !== 'ALL' && log.level.toUpperCase() !== levelFilter) return false;
      if (searchText) {
        const q = searchText.toLowerCase();
        return (
          log.message.toLowerCase().includes(q) ||
          (log.source ?? '').toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [logs, levelFilter, searchText]);

  const clearLogs = useCallback(() => setLogs([]), []);

  const getLevelBadgeStyle = (level: WsLogEntry['level']): React.CSSProperties => {
    switch (level) {
      case 'info': return { background: '#1e3a5f', color: '#60a5fa' };
      case 'warn': return { background: '#3d2e00', color: '#fbbf24' };
      case 'error': return { background: '#3d0a0a', color: '#f87171' };
      case 'debug': return { background: '#1f1f1f', color: '#9ca3af' };
    }
  };

  const formatTimestamp = (date: Date): string => {
    return date.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  };

  const isMock = effectiveMockMode;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', fontFamily: "var(--font-mono, 'JetBrains Mono', 'Fira Code', Consolas, monospace)" }}>
      {/* Toolbar */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 12px',
        borderBottom: '1px solid var(--border)',
        background: 'var(--background)',
        flexShrink: 0,
      }}>
        <IconLogs />
        <span style={{ fontSize: 13, fontWeight: 600, marginRight: 4 }}>Log Stream</span>

        {/* Connection status */}
        <StatusDot status={status} />
        {isMock && (
          <span style={{ fontSize: 10, color: 'var(--muted-foreground)', background: 'var(--accent)', padding: '1px 5px', borderRadius: 3 }}>
            DEMO
          </span>
        )}

        <span style={{ marginLeft: 'auto', fontSize: 11, color: 'var(--muted-foreground)' }}>
          {filteredLogs.length} / {logs.length}
        </span>

        {/* Search */}
        <input
          type="text"
          placeholder="Search logs..."
          value={searchText}
          onChange={e => setSearchText(e.target.value)}
          style={{
            background: 'var(--input)',
            border: '1px solid var(--border)',
            borderRadius: 4,
            padding: '3px 8px',
            fontSize: 12,
            fontFamily: 'inherit',
            color: 'var(--foreground)',
            width: 160,
          }}
        />

        {/* Level filter */}
        <select
          value={levelFilter}
          onChange={e => setLevelFilter(e.target.value as LevelFilter)}
          style={{
            background: 'var(--input)',
            border: '1px solid var(--border)',
            borderRadius: 4,
            padding: '3px 6px',
            fontSize: 12,
            fontFamily: 'inherit',
            color: 'var(--foreground)',
          }}
        >
          <option value="ALL">ALL</option>
          <option value="INFO">INFO</option>
          <option value="WARN">WARN</option>
          <option value="ERROR">ERROR</option>
          <option value="DEBUG">DEBUG</option>
        </select>

        {/* Auto-scroll toggle */}
        <button
          onClick={() => { setAutoScroll(v => !v); if (!autoScroll) userScrolledUpRef.current = false; }}
          title={autoScroll ? 'Pause auto-scroll' : 'Resume auto-scroll'}
          style={{
            background: autoScroll ? 'var(--primary)' : 'var(--accent)',
            color: autoScroll ? 'var(--primary-foreground)' : 'var(--foreground)',
            border: '1px solid var(--border)',
            borderRadius: 4,
            padding: '3px 8px',
            fontSize: 12,
            cursor: 'pointer',
            fontFamily: 'inherit',
          }}
        >
          {autoScroll ? 'Auto' : 'Paused'}
        </button>

        {/* Clear */}
        <button
          onClick={clearLogs}
          title="Clear logs"
          style={{
            background: 'transparent',
            border: '1px solid var(--border)',
            borderRadius: 4,
            padding: '3px 8px',
            fontSize: 12,
            cursor: 'pointer',
            color: 'var(--foreground)',
            fontFamily: 'inherit',
          }}
        >
          Clear
        </button>
      </div>

      {/* Log list */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '4px 0',
          fontSize: 12,
          lineHeight: 1.6,
          background: '#0d0d0d',
        }}
      >
        {filteredLogs.length === 0 ? (
          <div style={{ padding: '24px 16px', textAlign: 'center', color: '#555', fontSize: 13 }}>
            {logs.length === 0 ? 'Waiting for logs...' : 'No logs match the current filter.'}
          </div>
        ) : (
          filteredLogs.map(log => (
            <div
              key={log.id}
              style={{
                display: 'flex',
                gap: 8,
                padding: '1px 12px',
                borderBottom: '1px solid #1a1a1a',
              }}
            >
              {/* Timestamp */}
              <span style={{ color: '#555', flexShrink: 0, minWidth: 72 }}>
                {formatTimestamp(log.timestamp)}
              </span>

              {/* Level badge */}
              <span
                style={{
                  ...getLevelBadgeStyle(log.level),
                  borderRadius: 3,
                  padding: '0 5px',
                  fontSize: 10,
                  fontWeight: 700,
                  flexShrink: 0,
                  minWidth: 42,
                  textAlign: 'center',
                  lineHeight: '18px',
                }}
              >
                {log.level.toUpperCase()}
              </span>

              {/* Source */}
              {log.source && (
                <span style={{ color: '#888', flexShrink: 0 }}>
                  [{log.source}]
                </span>
              )}

              {/* Message */}
              <span style={{ color: '#ccc', wordBreak: 'break-all' }}>
                {log.message}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default LogViewer;
