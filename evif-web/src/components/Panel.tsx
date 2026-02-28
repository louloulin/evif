import React from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import Terminal from './Terminal';

export type PanelTab = 'terminal' | 'problems' | 'output';

interface ProblemItem {
  id: string;
  message: string;
  source?: string;
  severity?: 'error' | 'warning' | 'info';
}

interface PanelProps {
  activeTab: PanelTab | null;
  onTabChange: (tab: PanelTab) => void;
  problems: ProblemItem[];
  visible: boolean;
}

const Panel: React.FC<PanelProps> = ({
  activeTab,
  onTabChange,
  problems,
  visible
}) => {
  const currentTab = activeTab ?? 'terminal';

  if (!visible) return null;

  return (
    <div className="panel">
      <Tabs value={currentTab} onValueChange={(v) => onTabChange(v as PanelTab)} className="panel-tabs">
        <TabsList className="panel-tabs-list">
          <TabsTrigger value="terminal" className="panel-tab">终端</TabsTrigger>
          <TabsTrigger value="problems" className="panel-tab">
            问题
            {problems.length > 0 && (
              <span className="panel-tab-count">{problems.length}</span>
            )}
          </TabsTrigger>
          <TabsTrigger value="output" className="panel-tab">输出</TabsTrigger>
        </TabsList>
        <TabsContent value="terminal" className="panel-tab-content">
          <div className="panel-terminal">
            <Terminal />
          </div>
        </TabsContent>
        <TabsContent value="problems" className="panel-tab-content">
          <div className="panel-problems">
            {problems.length === 0 ? (
              <div className="panel-problems-empty">暂无问题</div>
            ) : (
              <ul className="panel-problems-list">
                {problems.map((p) => (
                  <li key={p.id} className={`panel-problems-item panel-problems-item--${p.severity ?? 'error'}`}>
                    <span className="panel-problems-message">{p.message}</span>
                    {p.source && <span className="panel-problems-source">{p.source}</span>}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </TabsContent>
        <TabsContent value="output" className="panel-tab-content">
          <div className="panel-output">
            <pre className="panel-output-pre">EVIF 2.2 输出日志将显示在此处。</pre>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default Panel;
export type { ProblemItem };
