import { useEffect, useRef, useCallback, useState } from 'react';

export type WebSocketStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting';

export interface UseWebSocketOptions {
  /** WebSocket URL (e.g. ws://host/ws/logs) */
  url?: string;
  /** Query string to append, e.g. "stream=logs" */
  query?: string;
  /** Maximum reconnect attempts. Defaults to 10. */
  maxReconnectAttempts?: number;
  /** Initial reconnect delay in ms. Defaults to 1000. */
  reconnectDelay?: number;
  /** Called with each parsed message object */
  onMessage?: (data: unknown) => void;
  /** Called on raw text message */
  onRawMessage?: (text: string) => void;
  /** Called on connection open */
  onOpen?: () => void;
  /** Called on connection error */
  onError?: (error: Event) => void;
  /** Called when status changes */
  onStatusChange?: (status: WebSocketStatus) => void;
  /** Set true to use mock/demo mode (simulated logs) */
  mockMode?: boolean;
}

export interface UseWebSocketReturn {
  status: WebSocketStatus;
  lastMessage: unknown;
  sendMessage: (data: unknown) => void;
  disconnect: () => void;
  reconnect: () => void;
}

const buildWsUrl = (url: string, query?: string): string => {
  const sep = url.includes('?') ? '&' : '?';
  return query ? `${url}${sep}${query}` : url;
};

const DEFAULT_WS_URL = () => {
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${window.location.host}/ws/logs`;
};

export function useWebSocket(options: UseWebSocketOptions = {}): UseWebSocketReturn {
  const {
    url,
    query = 'stream=logs',
    maxReconnectAttempts = 10,
    reconnectDelay = 1000,
    onMessage,
    onRawMessage,
    onOpen,
    onError,
    onStatusChange,
    mockMode = false,
  } = options;

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const disposedRef = useRef(false);
  const mockIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const [status, setStatus] = useState<WebSocketStatus>('disconnected');
  const [lastMessage, setLastMessage] = useState<unknown>(null);

  const setStatusState = useCallback((s: WebSocketStatus) => {
    setStatus(s);
    onStatusChange?.(s);
  }, [onStatusChange]);

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const disconnect = useCallback(() => {
    disposedRef.current = true;
    clearReconnectTimer();
    if (mockIntervalRef.current !== null) {
      clearInterval(mockIntervalRef.current);
      mockIntervalRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    setStatusState('disconnected');
  }, [clearReconnectTimer, setStatusState]);

  const connect = useCallback(() => {
    if (disposedRef.current) return;
    clearReconnectTimer();

    // ---- Mock/demo mode: simulate log streaming ----
    if (mockMode) {
      setStatusState('connecting');
      const timer = setTimeout(() => {
        if (disposedRef.current) return;
        setStatusState('connected');
        reconnectAttemptRef.current = 0;

        const sources = ['server', 'plugin', 'vfs', 'api', 'auth', 'websocket', 'fs', 'monitor'];
        const levels: Array<'INFO' | 'WARN' | 'ERROR' | 'DEBUG'> = ['INFO', 'WARN', 'ERROR', 'DEBUG'];
        const messages = [
          'Connection established',
          'Processing request...',
          'Cache hit for key user_session',
          'Slow query detected (>100ms)',
          'File uploaded successfully',
          'Authentication token refreshed',
          'Memory usage: 64.2%',
          'CPU usage spike detected: 95%',
          'Disk space running low: 15GB remaining',
          'Plugin "code-analysis" loaded',
          'WebSocket client connected',
          'Request timeout after 30s',
          'Rate limit exceeded for IP 192.168.1.1',
          'Config reloaded from /etc/evif.conf',
          'Graceful shutdown initiated',
        ];

        let counter = 0;
        mockIntervalRef.current = setInterval(() => {
          if (disposedRef.current) {
            clearInterval(mockIntervalRef.current!);
            mockIntervalRef.current = null;
            return;
          }
          const level = levels[Math.floor(Math.random() * levels.length)];
          const source = sources[Math.floor(Math.random() * sources.length)];
          const message = messages[Math.floor(Math.random() * messages.length)];
          const payload = {
            id: `mock-${Date.now()}-${counter++}`,
            timestamp: new Date().toISOString(),
            level: level.toLowerCase(),
            source,
            message: `${message} (${Math.random().toString(36).substring(2, 6)})`,
          };
          setLastMessage(payload);
          onMessage?.(payload);
          onRawMessage?.(JSON.stringify(payload));
        }, 1200 + Math.random() * 800);
      }, 600);
      return;
    }

    // ---- Real WebSocket mode ----
    setStatusState('connecting');
    const wsUrl = buildWsUrl(url ?? DEFAULT_WS_URL(), query);

    try {
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        if (disposedRef.current) {
          ws.close();
          return;
        }
        reconnectAttemptRef.current = 0;
        setStatusState('connected');
        onOpen?.();
      };

      ws.onmessage = (event) => {
        if (disposedRef.current) return;
        try {
          const data = JSON.parse(event.data);
          setLastMessage(data);
          onMessage?.(data);
        } catch {
          onRawMessage?.(event.data);
        }
      };

      ws.onerror = (error) => {
        console.error('[useWebSocket] error:', error);
        onError?.(error);
      };

      ws.onclose = () => {
        if (disposedRef.current) return;
        const attempt = reconnectAttemptRef.current;

        if (attempt < maxReconnectAttempts) {
          const delay = Math.min(reconnectDelay * Math.pow(2, attempt), 30000);
          setStatusState('reconnecting');
          reconnectAttemptRef.current = attempt + 1;
          reconnectTimerRef.current = setTimeout(() => {
            if (!disposedRef.current) {
              connect();
            }
          }, delay);
        } else {
          setStatusState('disconnected');
        }
      };
    } catch (err) {
      console.error('[useWebSocket] connection error:', err);
      setStatusState('disconnected');
    }
  }, [url, query, maxReconnectAttempts, reconnectDelay, mockMode, clearReconnectTimer, setStatusState, onMessage, onRawMessage, onOpen, onError]);

  const reconnect = useCallback(() => {
    disconnect();
    disposedRef.current = false;
    reconnectAttemptRef.current = 0;
    connect();
  }, [disconnect, connect]);

  // Connect on mount / url change
  useEffect(() => {
    connect();
    return () => {
      disposedRef.current = true;
      clearReconnectTimer();
      if (mockIntervalRef.current !== null) {
        clearInterval(mockIntervalRef.current);
        mockIntervalRef.current = null;
      }
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connect, clearReconnectTimer]);

  const sendMessage = useCallback((data: unknown) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(typeof data === 'string' ? data : JSON.stringify(data));
    }
  }, []);

  return { status, lastMessage, sendMessage, disconnect, reconnect };
}
