import React from 'react';

interface StatusBarProps {
  connected: boolean;
  currentFilePath: string | null;
}

const StatusBar: React.FC<StatusBarProps> = ({ connected, currentFilePath }) => {
  return (
    <footer className="status-bar" role="status">
      <div className="status-bar-left">
        <span className={`status-bar-item status-bar-connection ${connected ? 'connected' : 'disconnected'}`}>
          {connected ? '● 已连接' : '○ 未连接'}
        </span>
        {currentFilePath && (
          <span className="status-bar-item status-bar-path" title={currentFilePath}>
            {currentFilePath}
          </span>
        )}
      </div>
      <div className="status-bar-right">
        <span className="status-bar-item">EVIF 2.2</span>
      </div>
    </footer>
  );
};

export default StatusBar;
