import { useState, useEffect } from 'react';

interface NetworkStatus {
  isOnline: boolean;
  since?: Date;
}

/**
 * 网络状态检测Hook
 * 监听浏览器在线/离线事件,并进行健康检查轮询
 */
export function useNetworkStatus(): NetworkStatus {
  const [isOnline, setIsOnline] = useState(() => navigator.onLine);
  const [since, setSince] = useState<Date>();

  useEffect(() => {
    // 初始化状态
    setSince(new Date());

    const handleOnline = () => {
      setIsOnline(true);
      setSince(new Date());
    };

    const handleOffline = () => {
      setIsOnline(false);
      setSince(new Date());
    };

    // 监听原生事件
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // 轮询健康检查 (每5秒)
    const healthCheck = setInterval(async () => {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 3000);

        await fetch('/api/v1/health', {
          method: 'HEAD',
          signal: controller.signal,
        });

        clearTimeout(timeoutId);
        if (!isOnline) {
          handleOnline();
        }
      } catch (error) {
        if (isOnline) {
          handleOffline();
        }
      }
    }, 5000);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      clearInterval(healthCheck);
    };
  }, [isOnline]);

  return { isOnline, since };
}
