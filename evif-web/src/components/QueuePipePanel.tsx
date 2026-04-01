/**
 * QueuePipePanel - Queue and Pipe visualization view
 *
 * Displays QueueFS (FIFO queues) and PipeFS (pipes with status) in a tabbed interface.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { RefreshCw, ChevronDown, ChevronRight, Clock, User, ArrowDown, ArrowUp } from 'lucide-react';
import { httpFetch } from '@/lib/http';
import { Button } from './ui/button';
import { Badge } from './ui/badge';
import { ScrollArea } from './ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs';

interface QueueMessage {
  id: string;
  content: string;
  timestamp: string;
}

interface Queue {
  name: string;
  path: string;
  message_count: number;
  last_message?: QueueMessage;
  last_updated?: string;
}

type PipeStatus = 'pending' | 'running' | 'completed' | 'failed';

interface Pipe {
  name: string;
  path: string;
  status: PipeStatus;
  assignee?: string;
  timeout?: number;
  input?: string;
  output?: string;
  created_at?: string;
  updated_at?: string;
}

const statusColors: Record<PipeStatus, { bg: string; text: string; border: string }> = {
  pending: { bg: 'bg-gray-500/10', text: 'text-gray-400', border: 'border-gray-500/30' },
  running: { bg: 'bg-yellow-500/10', text: 'text-yellow-500', border: 'border-yellow-500/30' },
  completed: { bg: 'bg-green-500/10', text: 'text-green-500', border: 'border-green-500/30' },
  failed: { bg: 'bg-red-500/10', text: 'text-red-500', border: 'border-red-500/30' },
};

const QueuePipePanel: React.FC = () => {
  const [queues, setQueues] = useState<Queue[]>([]);
  const [pipes, setPipes] = useState<Pipe[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedQueue, setExpandedQueue] = useState<string | null>(null);
  const [expandedPipe, setExpandedPipe] = useState<string | null>(null);
  const [queueMessages, setQueueMessages] = useState<Record<string, QueueMessage[]>>({});
  const [loadingMessages, setLoadingMessages] = useState<Set<string>>(new Set());
  const [activeTab, setActiveTab] = useState<'queues' | 'pipes'>('queues');

  const fetchQueues = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await httpFetch('/api/v1/queue/list');
      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        throw new Error((data && (data.message || data.error)) || 'Failed to list queues');
      }

      const queueList: Queue[] = Array.isArray(data) ? data : (data.queues || []);
      setQueues(queueList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load queues');
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchPipes = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await httpFetch('/api/v1/pipe/list');
      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        throw new Error((data && (data.message || data.error)) || 'Failed to list pipes');
      }

      const pipeList: Pipe[] = Array.isArray(data) ? data : (data.pipes || []);
      setPipes(pipeList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pipes');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === 'queues') {
      fetchQueues();
    } else {
      fetchPipes();
    }
  }, [activeTab, fetchQueues, fetchPipes]);

  const fetchQueueMessages = async (queueName: string) => {
    setLoadingMessages(prev => new Set(prev).add(queueName));
    try {
      const response = await httpFetch(`/api/v1/queue/read?name=${encodeURIComponent(queueName)}`);
      const data = await response.json().catch(() => ({}));

      if (response.ok) {
        const messages: QueueMessage[] = Array.isArray(data) ? data : (data.messages || []);
        setQueueMessages(prev => ({ ...prev, [queueName]: messages }));
      }
    } catch {
      // Silently fail
    } finally {
      setLoadingMessages(prev => {
        const next = new Set(prev);
        next.delete(queueName);
        return next;
      });
    }
  };

  const handleEnqueue = async (queueName: string, content: string) => {
    try {
      const response = await httpFetch('/api/v1/queue/enqueue', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ queue: queueName, content }),
      });

      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error((data && (data.message || data.error)) || 'Enqueue failed');
      }

      fetchQueues();
      if (expandedQueue === queueName) {
        fetchQueueMessages(queueName);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Enqueue failed');
    }
  };

  const handleDequeue = async (queueName: string) => {
    try {
      const response = await httpFetch(`/api/v1/queue/dequeue?name=${encodeURIComponent(queueName)}`, {
        method: 'POST',
      });

      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error((data && (data.message || data.error)) || 'Dequeue failed');
      }

      fetchQueues();
      if (expandedQueue === queueName) {
        fetchQueueMessages(queueName);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Dequeue failed');
    }
  };

  const toggleQueueExpand = (queueName: string) => {
    if (expandedQueue === queueName) {
      setExpandedQueue(null);
    } else {
      setExpandedQueue(queueName);
      if (!queueMessages[queueName]) {
        fetchQueueMessages(queueName);
      }
    }
  };

  const togglePipeExpand = (pipeName: string) => {
    setExpandedPipe(prev => prev === pipeName ? null : pipeName);
  };

  const formatTimestamp = (ts?: string): string => {
    if (!ts) return '-';
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  };

  if (loading && queues.length === 0 && pipes.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Queue/Pipe</h2>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => activeTab === 'queues' ? fetchQueues() : fetchPipes()}
          title="Refresh"
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

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'queues' | 'pipes')} className="flex-1 flex flex-col">
        <TabsList className="w-full justify-start rounded-none border-b bg-muted/30 h-auto p-0">
          <TabsTrigger
            value="queues"
            className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2"
          >
            Queues
            <Badge variant="secondary" className="ml-2 text-xs">
              {queues.length}
            </Badge>
          </TabsTrigger>
          <TabsTrigger
            value="pipes"
            className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2"
          >
            Pipes
            <Badge variant="secondary" className="ml-2 text-xs">
              {pipes.length}
            </Badge>
          </TabsTrigger>
        </TabsList>

        <ScrollArea className="flex-1">
          {/* Queues Tab */}
          <TabsContent value="queues" className="m-0">
            <div className="p-3">
              {queues.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                  <p className="text-sm">No queues found</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {queues.map(queue => (
                    <div
                      key={queue.name}
                      className="rounded-lg border border-border bg-card overflow-hidden"
                    >
                      {/* Queue header */}
                      <button
                        className="w-full flex items-center gap-2 px-3 py-2.5 hover:bg-muted/30 transition-colors text-left"
                        onClick={() => toggleQueueExpand(queue.name)}
                      >
                        {expandedQueue === queue.name ? (
                          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                        ) : (
                          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                        )}
                        <span className="text-sm font-medium flex-1 truncate">{queue.name}</span>
                        <Badge variant="secondary" className="text-xs">
                          {queue.message_count} msg
                        </Badge>
                        {queue.last_updated && (
                          <span className="text-xs text-muted-foreground flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {formatTimestamp(queue.last_updated)}
                          </span>
                        )}
                      </button>

                      {/* Expanded content */}
                      {expandedQueue === queue.name && (
                        <div className="border-t border-border/50">
                          {/* Actions */}
                          <div className="px-3 py-2 flex items-center gap-2 border-b border-border/30">
                            <EnqueueForm queueName={queue.name} onEnqueue={handleEnqueue} />
                            <Button
                              variant="outline"
                              size="sm"
                              className="text-xs h-7"
                              onClick={() => handleDequeue(queue.name)}
                              disabled={queue.message_count === 0}
                            >
                              <ArrowDown className="h-3 w-3 mr-1" />
                              Dequeue
                            </Button>
                          </div>

                          {/* Last message preview */}
                          {queue.last_message && (
                            <div className="px-3 py-2 bg-muted/20">
                              <span className="text-xs text-muted-foreground">Last message:</span>
                              <pre className="text-xs font-mono mt-1 whitespace-pre-wrap break-all">
                                {queue.last_message.content}
                              </pre>
                            </div>
                          )}

                          {/* Messages list */}
                          <div className="px-3 py-2">
                            <div className="text-xs font-medium text-muted-foreground mb-1.5">
                              Messages
                              {loadingMessages.has(queue.name) && (
                                <span className="ml-2 inline-block animate-spin h-3 w-3 border-b-2 border-current rounded-full" />
                              )}
                            </div>
                            {queueMessages[queue.name] ? (
                              queueMessages[queue.name].length > 0 ? (
                                <div className="space-y-1.5">
                                  {queueMessages[queue.name].map((msg, idx) => (
                                    <div key={msg.id || idx} className="text-xs bg-muted/50 rounded p-2">
                                      <div className="flex items-center justify-between text-muted-foreground mb-1">
                                        <span>#{idx + 1}</span>
                                        <span>{formatTimestamp(msg.timestamp)}</span>
                                      </div>
                                      <pre className="whitespace-pre-wrap break-all font-mono">
                                        {msg.content}
                                      </pre>
                                    </div>
                                  ))}
                                </div>
                              ) : (
                                <p className="text-xs text-muted-foreground">No messages</p>
                              )
                            ) : (
                              <p className="text-xs text-muted-foreground">Click to load messages</p>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </TabsContent>

          {/* Pipes Tab */}
          <TabsContent value="pipes" className="m-0">
            <div className="p-3">
              {pipes.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                  <p className="text-sm">No pipes found</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {pipes.map(pipe => {
                    const colors = statusColors[pipe.status];
                    return (
                      <div
                        key={pipe.name}
                        className="rounded-lg border border-border bg-card overflow-hidden"
                      >
                        {/* Pipe header */}
                        <button
                          className="w-full flex items-center gap-2 px-3 py-2.5 hover:bg-muted/30 transition-colors text-left"
                          onClick={() => togglePipeExpand(pipe.name)}
                        >
                          {expandedPipe === pipe.name ? (
                            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                          ) : (
                            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                          )}
                          <span className="text-sm font-medium flex-1 truncate">{pipe.name}</span>
                          <Badge className={`${colors.bg} ${colors.text} text-xs border ${colors.border}`}>
                            {pipe.status}
                          </Badge>
                          {pipe.assignee && (
                            <span className="text-xs text-muted-foreground flex items-center gap-1">
                              <User className="h-3 w-3" />
                              {pipe.assignee}
                            </span>
                          )}
                          {pipe.timeout && (
                            <span className="text-xs text-muted-foreground flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {pipe.timeout}s
                            </span>
                          )}
                        </button>

                        {/* Expanded content */}
                        {expandedPipe === pipe.name && (
                          <div className="border-t border-border/50 px-3 py-2 space-y-2">
                            <div className="text-xs space-y-1">
                              {pipe.created_at && (
                                <div className="flex items-center gap-2 text-muted-foreground">
                                  <span>Created:</span>
                                  <span>{formatTimestamp(pipe.created_at)}</span>
                                </div>
                              )}
                              {pipe.updated_at && (
                                <div className="flex items-center gap-2 text-muted-foreground">
                                  <span>Updated:</span>
                                  <span>{formatTimestamp(pipe.updated_at)}</span>
                                </div>
                              )}
                            </div>

                            {pipe.input !== undefined && (
                              <div>
                                <span className="text-xs font-medium text-muted-foreground">Input:</span>
                                <pre className="text-xs font-mono bg-muted/50 rounded p-2 mt-1 whitespace-pre-wrap break-all max-h-[150px] overflow-auto">
                                  {pipe.input || '(empty)'}
                                </pre>
                              </div>
                            )}

                            {pipe.output !== undefined && (
                              <div>
                                <span className="text-xs font-medium text-muted-foreground">Output:</span>
                                <pre className="text-xs font-mono bg-muted/50 rounded p-2 mt-1 whitespace-pre-wrap break-all max-h-[150px] overflow-auto">
                                  {pipe.output || '(empty)'}
                                </pre>
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </TabsContent>
        </ScrollArea>
      </Tabs>
    </div>
  );
};

// Enqueue form component
interface EnqueueFormProps {
  queueName: string;
  onEnqueue: (queueName: string, content: string) => void;
}

const EnqueueForm: React.FC<EnqueueFormProps> = ({ queueName, onEnqueue }) => {
  const [content, setContent] = useState('');
  const [enqueuing, setEnqueuing] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;
    setEnqueuing(true);
    await onEnqueue(queueName, content.trim());
    setContent('');
    setEnqueuing(false);
  };

  return (
    <form onSubmit={handleSubmit} className="flex items-center gap-1 flex-1">
      <input
        type="text"
        value={content}
        onChange={(e) => setContent(e.target.value)}
        placeholder="Message..."
        className="flex-1 h-7 rounded-md border border-input bg-background px-2 py-1 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
      />
      <Button
        type="submit"
        variant="default"
        size="sm"
        className="text-xs h-7"
        disabled={!content.trim() || enqueuing}
      >
        <ArrowUp className="h-3 w-3 mr-1" />
        Enqueue
      </Button>
    </form>
  );
};

export default QueuePipePanel;
