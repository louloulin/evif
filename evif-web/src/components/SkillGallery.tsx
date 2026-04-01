/**
 * SkillGallery - Skill Gallery view
 *
 * Displays available skills in a card-based gallery with the ability
 * to view SKILL.md content, execute skills, and register new ones.
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Zap, Code, FileCode, Play, Plus, X, RefreshCw } from 'lucide-react';
import { httpFetch } from '@/lib/http';
import { Button } from './ui/button';
import { Badge } from './ui/badge';
import { ScrollArea } from './ui/scroll-area';
import { Textarea } from './ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from './ui/dialog';

interface Skill {
  name: string;
  path: string;
  description: string;
  triggers: string[];
  content: string;
}

const SkillGallery: React.FC = () => {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedSkill, setExpandedSkill] = useState<string | null>(null);

  // Execute dialog state
  const [executeDialogOpen, setExecuteDialogOpen] = useState(false);
  const [executeSkill, setExecuteSkill] = useState<Skill | null>(null);
  const [executeInput, setExecuteInput] = useState('');
  const [executeOutput, setExecuteOutput] = useState('');
  const [executing, setExecuting] = useState(false);

  // Register dialog state
  const [registerDialogOpen, setRegisterDialogOpen] = useState(false);
  const [registerName, setRegisterName] = useState('');
  const [registerContent, setRegisterContent] = useState('');
  const [registering, setRegistering] = useState(false);

  const fetchSkills = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const lsResponse = await httpFetch('/api/v1/fs/list?path=' + encodeURIComponent('/skills'));
      const lsData = await lsResponse.json().catch(() => ({}));

      if (!lsResponse.ok) {
        throw new Error((lsData && (lsData.message || lsData.error)) || 'Failed to list skills');
      }

      const nodes = lsData.nodes || [];
      const skillDirs = nodes.filter((n: { is_dir: boolean }) => n.is_dir);
      const loadedSkills: Skill[] = [];

      for (const dir of skillDirs) {
        try {
          const skillMdPath = `${dir.path}/SKILL.md`;
          const readResponse = await httpFetch(`/api/v1/fs/read?path=${encodeURIComponent(skillMdPath)}`);
          const readData = await readResponse.json().catch(() => ({}));

          if (readResponse.ok && readData.content) {
            const content = readData.content as string;
            const description = extractDescription(content);
            const triggers = extractTriggers(content);

            loadedSkills.push({
              name: dir.name,
              path: dir.path,
              description,
              triggers,
              content,
            });
          }
        } catch {
          // Skip skills that fail to load
        }
      }

      setSkills(loadedSkills);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load skills');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSkills();
  }, [fetchSkills]);

  /**
   * Extract a short description from SKILL.md content
   */
  const extractDescription = (content: string): string => {
    const lines = content.split('\n');
    // Look for the first non-heading, non-empty line
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line && !line.startsWith('#')) {
        return line.length > 120 ? line.substring(0, 120) + '...' : line;
      }
    }
    return 'No description available';
  };

  /**
   * Extract trigger keywords from SKILL.md content
   */
  const extractTriggers = (content: string): string[] => {
    const triggers: string[] = [];
    const triggerMatch = content.match(/(?:trigger|trigger_words|keywords|triggers)[:\s]*\n?([\s\S]*?)(?:\n#|\n$)/i);
    if (triggerMatch) {
      const lines = triggerMatch[1].split('\n');
      for (const line of lines) {
        const cleaned = line.replace(/^[-*]\s*/, '').trim().replace(/`/g, '');
        if (cleaned) triggers.push(cleaned);
      }
    }

    // If no explicit triggers found, use the skill name
    if (triggers.length === 0) {
      triggers.push('general');
    }

    return triggers;
  };

  const handleExecute = (skill: Skill) => {
    setExecuteSkill(skill);
    setExecuteInput('');
    setExecuteOutput('');
    setExecuteDialogOpen(true);
  };

  const handleRunExecute = async () => {
    if (!executeSkill) return;
    setExecuting(true);
    setExecuteOutput('');

    try {
      // Try calling the skill execution API
      const response = await httpFetch('/api/v1/skills/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          skill: executeSkill.name,
          input: executeInput,
        }),
      });

      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        throw new Error((data && (data.message || data.error)) || 'Execution failed');
      }

      setExecuteOutput(data.output || data.result || JSON.stringify(data, null, 2));
    } catch (err) {
      setExecuteOutput(`Error: ${err instanceof Error ? err.message : 'Execution failed'}`);
    } finally {
      setExecuting(false);
    }
  };

  const handleRegister = async () => {
    if (!registerName.trim() || !registerContent.trim()) return;
    setRegistering(true);

    try {
      const skillPath = `/skills/${registerName.trim()}/SKILL.md`;
      const response = await httpFetch(`/api/v1/fs/write?path=${encodeURIComponent(skillPath)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: registerContent }),
      });

      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error((data && (data.message || data.error)) || 'Registration failed');
      }

      setRegisterDialogOpen(false);
      setRegisterName('');
      setRegisterContent('');
      fetchSkills();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Registration failed');
    } finally {
      setRegistering(false);
    }
  };

  const toggleExpanded = (skillName: string) => {
    setExpandedSkill(prev => prev === skillName ? null : skillName);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
          <p className="text-sm text-muted-foreground">Loading skills...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <Zap className="h-4 w-4 text-primary" />
          <h2 className="text-sm font-semibold">Skill Gallery</h2>
          <Badge variant="secondary" className="text-xs">
            {skills.length}
          </Badge>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            onClick={fetchSkills}
            title="Refresh skills"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setRegisterDialogOpen(true)}
          >
            <Plus className="h-3.5 w-3.5 mr-1.5" />
            Register New
          </Button>
        </div>
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

      {/* Skills grid */}
      <ScrollArea className="flex-1">
        <div className="p-3">
          {skills.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Code className="h-10 w-10 mb-3 opacity-50" />
              <p className="text-sm mb-1">No skills found</p>
              <p className="text-xs">Register a new skill to get started</p>
              <Button
                variant="outline"
                size="sm"
                className="mt-4"
                onClick={() => setRegisterDialogOpen(true)}
              >
                <Plus className="h-3.5 w-3.5 mr-1.5" />
                Register New Skill
              </Button>
            </div>
          ) : (
            <div className="space-y-2.5">
              {skills.map(skill => (
                <div
                  key={skill.name}
                  className="rounded-lg border border-border bg-card overflow-hidden"
                >
                  {/* Skill card header */}
                  <button
                    className="w-full px-3 py-2.5 text-left hover:bg-muted/30 transition-colors"
                    onClick={() => toggleExpanded(skill.name)}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <FileCode className="h-3.5 w-3.5 text-primary flex-shrink-0" />
                          <span className="text-sm font-semibold truncate">{skill.name}</span>
                        </div>
                        <p className="text-xs text-muted-foreground line-clamp-2">
                          {skill.description}
                        </p>
                        {skill.triggers.length > 0 && (
                          <div className="flex flex-wrap gap-1 mt-1.5">
                            {skill.triggers.slice(0, 4).map(trigger => (
                              <Badge
                                key={trigger}
                                variant="outline"
                                className="text-[10px] px-1.5 py-0 h-4"
                              >
                                {trigger}
                              </Badge>
                            ))}
                            {skill.triggers.length > 4 && (
                              <Badge
                                variant="outline"
                                className="text-[10px] px-1.5 py-0 h-4"
                              >
                                +{skill.triggers.length - 4}
                              </Badge>
                            )}
                          </div>
                        )}
                      </div>
                      <div className="flex items-center gap-1 flex-shrink-0">
                        <Button
                          variant="default"
                          size="sm"
                          className="text-xs h-7"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleExecute(skill);
                          }}
                        >
                          <Play className="h-3 w-3 mr-1" />
                          Execute
                        </Button>
                      </div>
                    </div>
                  </button>

                  {/* Expanded skill content */}
                  {expandedSkill === skill.name && (
                    <div className="border-t border-border">
                      <div className="px-3 py-2">
                        <div className="flex items-center justify-between mb-1.5">
                          <span className="text-xs font-medium text-muted-foreground">SKILL.md</span>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-6 text-xs"
                            onClick={() => setExpandedSkill(null)}
                          >
                            <X className="h-3 w-3 mr-1" />
                            Close
                          </Button>
                        </div>
                        <pre className="text-xs font-mono bg-muted/50 rounded p-2.5 overflow-x-auto max-h-[300px] overflow-y-auto whitespace-pre-wrap break-all">
                          {skill.content}
                        </pre>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Execute Dialog */}
      <Dialog open={executeDialogOpen} onOpenChange={setExecuteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Play className="h-4 w-4" />
              Execute: {executeSkill?.name}
            </DialogTitle>
            <DialogDescription>
              Provide input for the skill and click Run to execute it.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div>
              <label className="text-sm font-medium mb-1.5 block">Input</label>
              <Textarea
                value={executeInput}
                onChange={(e) => setExecuteInput(e.target.value)}
                placeholder="Enter input for the skill..."
                className="text-xs font-mono min-h-[100px] resize-y"
              />
            </div>
            {executeOutput && (
              <div>
                <label className="text-sm font-medium mb-1.5 block">Output</label>
                <pre className="text-xs font-mono bg-muted/50 rounded p-2.5 max-h-[200px] overflow-auto whitespace-pre-wrap break-all">
                  {executeOutput}
                </pre>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setExecuteDialogOpen(false)}
              disabled={executing}
            >
              Close
            </Button>
            <Button
              onClick={handleRunExecute}
              disabled={executing}
            >
              {executing ? (
                <>
                  <div className="inline-block animate-spin rounded-full h-3 w-3 border-b-2 border-current mr-2" />
                  Running...
                </>
              ) : (
                <>
                  <Play className="h-3.5 w-3.5 mr-1.5" />
                  Run
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Register Dialog */}
      <Dialog open={registerDialogOpen} onOpenChange={setRegisterDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Plus className="h-4 w-4" />
              Register New Skill
            </DialogTitle>
            <DialogDescription>
              Create a new skill by providing a name and the SKILL.md content.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div>
              <label className="text-sm font-medium mb-1.5 block">Skill Name</label>
              <input
                type="text"
                value={registerName}
                onChange={(e) => setRegisterName(e.target.value)}
                placeholder="e.g., code-review"
                className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
            <div>
              <label className="text-sm font-medium mb-1.5 block">SKILL.md Content</label>
              <Textarea
                value={registerContent}
                onChange={(e) => setRegisterContent(e.target.value)}
                placeholder={`# My Skill\n\nDescription of what this skill does.\n\n## Triggers\n- keyword1\n- keyword2\n\n## Instructions\nStep-by-step instructions...`}
                className="text-xs font-mono min-h-[200px] resize-y"
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setRegisterDialogOpen(false)}
              disabled={registering}
            >
              Cancel
            </Button>
            <Button
              onClick={handleRegister}
              disabled={registering || !registerName.trim() || !registerContent.trim()}
            >
              {registering ? (
                <>
                  <div className="inline-block animate-spin rounded-full h-3 w-3 border-b-2 border-current mr-2" />
                  Registering...
                </>
              ) : (
                <>
                  <Plus className="h-3.5 w-3.5 mr-1.5" />
                  Register
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default SkillGallery;
