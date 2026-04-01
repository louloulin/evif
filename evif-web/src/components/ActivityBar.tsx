import React from 'react';

const IconExplorer = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
    <line x1="12" y1="11" x2="12" y2="17" />
    <line x1="9" y1="14" x2="15" y2="14" />
  </svg>
);

const IconTerminal = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polyline points="4 17 10 11 4 5" />
    <line x1="12" y1="19" x2="20" y2="19" />
  </svg>
);

const IconProblems = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="12" />
    <line x1="12" y1="16" x2="12.01" y2="16" />
  </svg>
);

const IconPlugins = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2v4" /><path d="M12 18v4" /><path d="M4.93 4.93l2.83 2.83" /><path d="M16.24 16.24l2.83 2.83" /><path d="M2 12h4" /><path d="M18 12h4" /><path d="M4.93 19.07l2.83-2.83" /><path d="M16.24 7.76l2.83-2.83" /><circle cx="12" cy="12" r="3" />
  </svg>
);

const IconSearch = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
  </svg>
);

const IconMonitor = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect width="20" height="14" x="2" y="3" rx="2" /><line x1="8" x2="16" y1="21" y2="21" /><line x1="12" x2="12" y1="17" y2="21" />
  </svg>
);

const IconMemory = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2a10 10 0 1 0 10 10" />
    <path d="M12 12l8-8" />
    <circle cx="12" cy="12" r="4" />
  </svg>
);

const IconContext = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2a8 8 0 0 0-8 8c0 3.4 2.1 6.3 5 7.5V20h6v-2.5c2.9-1.2 5-4.1 5-7.5a8 8 0 0 0-8-8z" />
    <path d="M10 22h4" />
    <path d="M9 12h6" />
    <path d="M12 9v6" />
  </svg>
);

const IconSkills = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
  </svg>
);

const IconQueuePipe = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 6h16" />
    <path d="M4 12h16" />
    <path d="M4 18h10" />
    <circle cx="19" cy="18" r="2" fill="currentColor" />
  </svg>
);

const IconLogs = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
    <line x1="8" y1="21" x2="16" y2="21" />
    <line x1="12" y1="17" x2="12" y2="21" />
    <line x1="6" y1="8" x2="14" y2="8" />
    <line x1="6" y1="11" x2="10" y2="11" />
  </svg>
);

export type ActivityView = 'explorer' | 'terminal' | 'problems' | 'plugins' | 'search' | 'monitor' | 'memory' | 'context' | 'skills' | 'queue' | 'logs';

interface ActivityBarProps {
  activeView: ActivityView | null;
  onViewChange: (view: ActivityView) => void;
  sidebarVisible: boolean;
  panelVisible: boolean;
  problemsCount?: number;
}

const ActivityBar: React.FC<ActivityBarProps> = ({
  activeView,
  onViewChange,
  sidebarVisible,
  panelVisible,
  problemsCount = 0
}) => {
  const handleExplorer = () => {
    onViewChange('explorer');
  };
  const handleTerminal = () => {
    onViewChange('terminal');
  };
  const handleProblems = () => {
    onViewChange('problems');
  };
  const handlePlugins = () => {
    onViewChange('plugins');
  };
  const handleSearch = () => {
    onViewChange('search');
  };
  const handleMonitor = () => {
    onViewChange('monitor');
  };
  const handleMemory = () => {
    onViewChange('memory');
  };
  const handleContext = () => {
    onViewChange('context');
  };
  const handleSkills = () => {
    onViewChange('skills');
  };
  const handleQueue = () => {
    onViewChange('queue');
  };
  const handleLogs = () => {
    onViewChange('logs');
  };

  return (
    <div className="activity-bar" aria-label="活动栏">
      <div className="activity-bar-inner">
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'explorer' && sidebarVisible ? 'active' : ''}`}
          onClick={handleExplorer}
          title="资源管理器 (Ctrl+Shift+E)"
        >
          <IconExplorer />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'terminal' && panelVisible ? 'active' : ''}`}
          onClick={handleTerminal}
          title="终端 (Ctrl+`)"
        >
          <IconTerminal />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'problems' && panelVisible ? 'active' : ''}`}
          onClick={handleProblems}
          title="问题"
        >
          <IconProblems />
          {problemsCount > 0 && (
            <span className="activity-bar-badge">{problemsCount > 99 ? '99+' : problemsCount}</span>
          )}
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'plugins' && sidebarVisible ? 'active' : ''}`}
          onClick={handlePlugins}
          title="插件管理"
        >
          <IconPlugins />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'search' && sidebarVisible ? 'active' : ''}`}
          onClick={handleSearch}
          title="搜索与上传"
        >
          <IconSearch />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'monitor' && sidebarVisible ? 'active' : ''}`}
          onClick={handleMonitor}
          title="系统监控"
        >
          <IconMonitor />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'memory' && sidebarVisible ? 'active' : ''}`}
          onClick={handleMemory}
          title="记忆管理"
        >
          <IconMemory />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'context' && sidebarVisible ? 'active' : ''}`}
          onClick={handleContext}
          title="Context Explorer"
        >
          <IconContext />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'skills' && sidebarVisible ? 'active' : ''}`}
          onClick={handleSkills}
          title="Skill Gallery"
        >
          <IconSkills />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'queue' && sidebarVisible ? 'active' : ''}`}
          onClick={handleQueue}
          title="Queue/Pipe"
        >
          <IconQueuePipe />
        </button>
        <button
          type="button"
          className={`activity-bar-item ${activeView === 'logs' && sidebarVisible ? 'active' : ''}`}
          onClick={handleLogs}
          title="Log Stream"
        >
          <IconLogs />
        </button>
      </div>
    </div>
  );
};

export default ActivityBar;
