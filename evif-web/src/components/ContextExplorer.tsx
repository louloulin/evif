/**
 * ContextExplorer - Context Explorer view
 *
 * Visualizes the L0/L1/L2 context layers with collapsible sections,
 * file previews, inline editing, and token usage indicators.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Brain, Layers, BookOpen, FileText, Plus, RefreshCw, ChevronDown, ChevronRight, Save, X } from 'lucide-react';
import { httpFetch } from '@/lib/http';
import { Button } from './ui/button';
import { ScrollArea } from './ui/scroll-area';
import { Textarea } from './ui/textarea';

interface ContextFile {
  path: string;
  name: string;
  content: string;
}

interface ContextLayer {
  id: string;
  label: string;
  sublabel: string;
  color: string;
  bgColor: string;
  borderColor: string;
  icon: React.ReactNode;
  files: ContextFile[];
  totalTokens: number;
}

const ContextExplorer: React.FC = () => {
  const [expandedLayers, setExpandedLayers] = useState<Set<string>>(new Set(['L0', 'L1', 'L2']));
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set());
  const [layers, setLayers] = useState<ContextLayer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [editContent, setEditContent] = useState('');
  const [saving, setSaving] = useState(false);
  const [newDecision, setNewDecision] = useState('');
  const [addingDecision, setAddingDecision] = useState(false);

  const CONTEXT_PATHS = {
    L0: '/context/L0',
    L1: '/context/L1',
    L2: '/context/L2',
  };

  const estimateTokens = (text: string): number => {
    return Math.ceil(text.length / 4);
  };

  const buildLayers = useCallback(async () => {
    setLoading(true);
    setError(null);

    const layerConfigs = [
      {
        id: 'L0',
        label: 'L0 - Instant',
        sublabel: 'Current working context',
        color: 'text-blue-500',
        bgColor: 'bg-blue-500/10',
        borderColor: 'border-blue-500/30',
        icon: <Brain className="h-4 w-4" />,
        path: CONTEXT_PATHS.L0,
      },
      {
        id: 'L1',
        label: 'L1 - Session',
        sublabel: 'Session decisions & notes',
        color: 'text-yellow-500',
        bgColor: 'bg-yellow-500/10',
        borderColor: 'border-yellow-500/30',
        icon: <Layers className="h-4 w-4" />,
        path: CONTEXT_PATHS.L1,
      },
      {
        id: 'L2',
        label: 'L2 - Knowledge',
        sublabel: 'Long-term knowledge base',
        color: 'text-green-500',
        bgColor: 'bg-green-500/10',
        borderColor: 'border-green-500/30',
        icon: <BookOpen className="h-4 w-4" />,
        path: CONTEXT_PATHS.L2,
      },
    ];

    const result: ContextLayer[] = [];

    for (const config of layerConfigs) {
      const files: ContextFile[] = [];
      let totalTokens = 0;

      try {
        const lsResponse = await httpFetch(`/api/v1/fs/list?path=${encodeURIComponent(config.path)}`);
        const lsData = await lsResponse.json().catch(() => ({}));

        if (lsResponse.ok && lsData.nodes) {
          const fileNodes = lsData.nodes.filter((n: { is_dir: boolean }) => !n.is_dir);

          for (const node of fileNodes) {
            try {
              const readResponse = await httpFetch(`/api/v1/fs/read?path=${encodeURIComponent(node.path)}`);
              const readData = await readResponse.json().catch(() => ({}));

              if (readResponse.ok) {
                const content = readData.content ?? '';
                totalTokens += estimateTokens(content);
                files.push({
                  path: node.path,
                  name: node.name,
                  content,
                });
              }
            } catch {
              // Skip files that fail to read
            }
          }
        }
      } catch {
        // Layer directory may not exist yet
      }

      result.push({
        id: config.id,
        label: config.label,
        sublabel: config.sublabel,
        color: config.color,
        bgColor: config.bgColor,
        borderColor: config.borderColor,
        icon: config.icon,
        files,
        totalTokens,
      });
    }

    setLayers(result);
    setLoading(false);
  }, []);

  useEffect(() => {
    buildLayers();
  }, [buildLayers]);

  const toggleLayer = (layerId: string) => {
    setExpandedLayers(prev => {
      const next = new Set(prev);
      if (next.has(layerId)) {
        next.delete(layerId);
      } else {
        next.add(layerId);
      }
      return next;
    });
  };

  const toggleFile = (filePath: string) => {
    setExpandedFiles(prev => {
      const next = new Set(prev);
      if (next.has(filePath)) {
        next.delete(filePath);
        if (editingFile === filePath) {
          setEditingFile(null);
        }
      } else {
        next.add(filePath);
      }
      return next;
    });
  };

  const handleStartEdit = (filePath: string, content: string) => {
    setEditingFile(filePath);
    setEditContent(content);
  };

  const handleSaveEdit = async (filePath: string) => {
    setSaving(true);
    try {
      const response = await httpFetch(`/api/v1/fs/write?path=${encodeURIComponent(filePath)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: editContent }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error((data && (data.message || data.error)) || 'Save failed');
      }

      setEditingFile(null);
      buildLayers();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleAddDecision = async () => {
    if (!newDecision.trim()) return;
    setAddingDecision(true);

    try {
      // Find the decisions.md file in L1, or create it
      const l1Layer = layers.find(l => l.id === 'L1');
      let decisionsFile = l1Layer?.files.find(f => f.name === 'decisions.md');
      let content = decisionsFile?.content ?? '';
      const timestamp = new Date().toISOString();
      content += `\n## Decision - ${timestamp}\n${newDecision.trim()}\n`;

      const targetPath = decisionsFile?.path ?? '/context/L1/decisions.md';
      const response = await httpFetch(`/api/v1/fs/write?path=${encodeURIComponent(targetPath)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error((data && (data.message || data.error)) || 'Failed to add decision');
      }

      setNewDecision('');
      buildLayers();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add decision');
    } finally {
      setAddingDecision(false);
    }
  };

  const handleUpdateCurrent = async () => {
    // Refresh L0 current context
    buildLayers();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
          <p className="text-sm text-muted-foreground">Loading context layers...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-primary" />
          <h2 className="text-sm font-semibold">Context Explorer</h2>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={buildLayers}
          title="Refresh context layers"
        >
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="px-4 py-2 bg-destructive/10 text-destructive text-xs border-b">
          {error}
          <button
            className="ml-2 underline"
            onClick={() => setError(null)}
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Layer sections */}
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-3">
          {layers.map(layer => (
            <div
              key={layer.id}
              className={`rounded-lg border ${layer.borderColor} overflow-hidden`}
            >
              {/* Layer header */}
              <button
                className={`w-full flex items-center gap-2 px-3 py-2.5 ${layer.bgColor} hover:opacity-80 transition-opacity`}
                onClick={() => toggleLayer(layer.id)}
              >
                {expandedLayers.has(layer.id) ? (
                  <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                )}
                <span className={layer.color}>{layer.icon}</span>
                <span className="text-sm font-medium flex-1 text-left">{layer.label}</span>
                <span className="text-xs text-muted-foreground">{layer.totalTokens} tokens</span>
              </button>

              {/* Layer content */}
              {expandedLayers.has(layer.id) && (
                <div className="border-t border-border/50">
                  <div className="px-3 py-1.5 text-xs text-muted-foreground bg-muted/20">
                    {layer.sublabel}
                  </div>

                  {layer.files.length === 0 ? (
                    <div className="px-3 py-4 text-xs text-muted-foreground text-center">
                      No files in this layer
                    </div>
                  ) : (
                    <div className="divide-y divide-border/30">
                      {layer.files.map(file => (
                        <div key={file.path}>
                          {/* File row */}
                          <button
                            className="w-full flex items-center gap-2 px-3 py-2 hover:bg-muted/30 transition-colors text-left"
                            onClick={() => toggleFile(file.path)}
                          >
                            {expandedFiles.has(file.path) ? (
                              <ChevronDown className="h-3 w-3 text-muted-foreground flex-shrink-0" />
                            ) : (
                              <ChevronRight className="h-3 w-3 text-muted-foreground flex-shrink-0" />
                            )}
                            <FileText className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                            <span className="text-xs font-medium truncate flex-1">{file.name}</span>
                            <span className="text-xs text-muted-foreground flex-shrink-0">
                              {estimateTokens(file.content)}t
                            </span>
                          </button>

                          {/* Expanded file content */}
                          {expandedFiles.has(file.path) && (
                            <div className="px-3 pb-2">
                              {editingFile === file.path ? (
                                /* Inline edit mode */
                                <div className="space-y-2">
                                  <Textarea
                                    value={editContent}
                                    onChange={(e) => setEditContent(e.target.value)}
                                    className="text-xs font-mono min-h-[120px] resize-y"
                                    placeholder="File content..."
                                  />
                                  <div className="flex items-center gap-2 justify-end">
                                    <Button
                                      variant="ghost"
                                      size="sm"
                                      onClick={() => setEditingFile(null)}
                                      disabled={saving}
                                    >
                                      <X className="h-3 w-3 mr-1" />
                                      Cancel
                                    </Button>
                                    <Button
                                      size="sm"
                                      onClick={() => handleSaveEdit(file.path)}
                                      disabled={saving}
                                    >
                                      <Save className="h-3 w-3 mr-1" />
                                      {saving ? 'Saving...' : 'Save'}
                                    </Button>
                                  </div>
                                </div>
                              ) : (
                                /* Read-only preview */
                                <div className="relative">
                                  <pre className="text-xs font-mono bg-muted/50 rounded p-2 overflow-x-auto max-h-[200px] overflow-y-auto whitespace-pre-wrap break-all">
                                    {file.content || '(empty file)'}
                                  </pre>
                                  {(layer.id === 'L0' || layer.id === 'L1') && (
                                    <div className="mt-1.5 flex justify-end">
                                      <Button
                                        variant="ghost"
                                        size="sm"
                                        onClick={() => handleStartEdit(file.path, file.content)}
                                      >
                                        Edit
                                      </Button>
                                    </div>
                                  )}
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}

                  {/* Layer-specific actions */}
                  <div className="px-3 py-2 border-t border-border/30">
                    {layer.id === 'L0' && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="w-full text-xs"
                        onClick={handleUpdateCurrent}
                      >
                        <RefreshCw className="h-3 w-3 mr-1.5" />
                        Update Current
                      </Button>
                    )}

                    {layer.id === 'L1' && (
                      <div className="space-y-2">
                        {addingDecision ? (
                          <div className="space-y-2">
                            <Textarea
                              value={newDecision}
                              onChange={(e) => setNewDecision(e.target.value)}
                              placeholder="Enter decision text..."
                              className="text-xs min-h-[80px] resize-y"
                            />
                            <div className="flex gap-2 justify-end">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => { setAddingDecision(false); setNewDecision(''); }}
                              >
                                Cancel
                              </Button>
                              <Button
                                size="sm"
                                onClick={handleAddDecision}
                                disabled={!newDecision.trim()}
                              >
                                Save Decision
                              </Button>
                            </div>
                          </div>
                        ) : (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="w-full text-xs"
                            onClick={() => setAddingDecision(true)}
                          >
                            <Plus className="h-3 w-3 mr-1.5" />
                            Add Decision
                          </Button>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
};

export default ContextExplorer;
