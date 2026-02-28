import React from 'react';
import { Button } from '@/components/ui/button';

interface MenuBarProps {
  onRefresh: () => void;
  onNewFile: () => void;
  onToggleTerminal: () => void;
  onToggleSidebar: () => void;
  /** 无挂载时禁用新建文件 */
  newFileDisabled?: boolean;
}

const IconRefresh = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path d="M3 3v5h5" /><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" /><path d="M16 21h5v-5" /></svg>
);
const IconPlus = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12h14" /><path d="M12 5v14" /></svg>
);
const IconTerminal = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" /></svg>
);
const IconSidebar = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="7" height="18" rx="1" /><rect x="14" y="3" width="7" height="18" rx="1" /></svg>
);

const MenuBar: React.FC<MenuBarProps> = ({
  onRefresh,
  onNewFile,
  onToggleTerminal,
  onToggleSidebar,
  newFileDisabled = false
}) => {
  return (
    <header className="flex items-center justify-between h-11 md:h-12 lg:h-14 shrink-0 px-3 md:px-4 lg:px-6 border-b border-border bg-card/80 backdrop-blur-sm">
      <div className="flex items-center gap-2 text-sm md:text-base lg:text-lg font-semibold text-foreground tracking-tight">
        EVIF 2.2
      </div>
      <div className="flex items-center gap-1.5 md:gap-2 lg:gap-4">
        <Button variant="ghost" size="icon" className="h-9 w-9 md:h-10 md:w-10 lg:h-11 lg:w-11 min-h-[44px] min-w-[44px] text-muted-foreground hover:text-foreground active:scale-95 transition-transform" onClick={onRefresh} title="刷新">
          <IconRefresh />
        </Button>
        <Button variant="ghost" size="sm" className="h-9 md:h-10 lg:h-11 gap-1.5 md:gap-2 px-3 md:px-4 min-h-[44px] text-muted-foreground hover:text-foreground disabled:opacity-50 active:scale-95 transition-transform" onClick={onNewFile} title={newFileDisabled ? '请先配置挂载点' : '新建文件'} disabled={newFileDisabled}>
          <span className="shrink-0"><IconPlus /></span>
          <span className="text-xs sm:text-sm hidden sm:inline">新建文件</span>
        </Button>
        <Button variant="ghost" size="icon" className="h-9 w-9 md:h-10 md:w-10 lg:h-11 lg:w-11 min-h-[44px] min-w-[44px] text-muted-foreground hover:text-foreground active:scale-95 transition-transform" onClick={onToggleTerminal} title="切换终端">
          <IconTerminal />
        </Button>
        <Button variant="ghost" size="icon" className="h-9 w-9 md:h-10 md:w-10 lg:h-11 lg:w-11 min-h-[44px] min-w-[44px] text-muted-foreground hover:text-foreground active:scale-95 transition-transform hidden md:flex" onClick={onToggleSidebar} title="切换侧边栏">
          <IconSidebar />
        </Button>
      </div>
    </header>
  );
};

export default MenuBar;
