import { useNetworkStatus } from '../hooks/useNetworkStatus';
import { WifiOff, CheckCircle2 } from 'lucide-react';
import { useEffect, useState } from 'react';

/**
 * 网络状态横幅组件
 * 在页面顶部显示网络连接状态
 */
export function NetworkBanner() {
  const { isOnline, since } = useNetworkStatus();
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!isOnline) {
      // 立即显示离线横幅
      setVisible(true);
    } else {
      // 网络恢复后3秒隐藏
      const timer = setTimeout(() => {
        setVisible(false);
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [isOnline]);

  if (!visible) return null;

  const isOffline = !isOnline;

  return (
    <div
      className={`network-banner ${isOffline ? 'offline' : 'online'}`}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 50,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '0.5rem',
        padding: '0.75rem 1rem',
        fontSize: '0.875rem',
        fontWeight: 500,
        transition: 'all 150ms ease-in-out',
        background: isOffline
          ? 'hsl(var(--destructive) / 0.1)'
          : 'hsl(var(--primary) / 0.1)',
        borderBottom: isOffline
          ? '1px solid hsl(var(--destructive) / 0.3)'
          : '1px solid hsl(var(--primary) / 0.3)',
        color: isOffline
          ? 'hsl(var(--destructive))'
          : 'hsl(var(--primary))',
      }}
    >
      {isOffline ? (
        <>
          <WifiOff className="h-4 w-4" />
          <span>网络连接已断开,部分功能可能不可用</span>
        </>
      ) : (
        <>
          <CheckCircle2 className="h-4 w-4" />
          <span>网络已恢复</span>
        </>
      )}
    </div>
  );
}
