import React, { useEffect, useRef, forwardRef, useImperativeHandle } from 'react';
import { Editor as MonacoEditor } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { SkeletonEditor } from './ui/skeleton';

interface FileNode {
  path: string;
  name: string;
  is_dir: boolean;
  children?: FileNode[];
}

interface EditorProps {
  file: FileNode | null;
  content: string;
  onSave: (content: string) => void;
  onChange?: (content: string) => void;
  filesError?: string | null;
  onRetry?: () => void;
  loading?: boolean;
}

export interface EditorRef {
  save: () => void;
}

const Editor = forwardRef<EditorRef, EditorProps>(({ file, content, onSave, onChange, filesError, onRetry, loading = false }, ref) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);

  // Expose save method to parent via ref
  useImperativeHandle(ref, () => ({
    save: () => {
      if (editorRef.current) {
        const value = editorRef.current.getValue();
        onSave(value);
      }
    }
  }));

  const handleEditorDidMount = (editor: editor.IStandaloneCodeEditor, monaco: any) => {
    editorRef.current = editor;

    // Add save shortcut (Ctrl+S / Cmd+S)
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      const value = editor.getValue();
      onSave(value);
    });

    // Configure keyboard shortcuts
    editor.addAction({
      id: 'save-file',
      label: '保存',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      contextMenuGroupId: 'navigation',
      run: () => {
        const value = editor.getValue();
        onSave(value);
      }
    });
  };

  const handleEditorChange = (value: string | undefined) => {
    if (onChange && value !== undefined) {
      onChange(value);
    }
  };

  const getLanguageFromFilename = (filename: string): string => {
    if (!filename) return 'plaintext';
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    const languageMap: Record<string, string> = {
      js: 'javascript',
      jsx: 'javascript',
      ts: 'typescript',
      tsx: 'typescript',
      py: 'python',
      java: 'java',
      c: 'c',
      cpp: 'cpp',
      cs: 'csharp',
      php: 'php',
      rb: 'ruby',
      go: 'go',
      rs: 'rust',
      sql: 'sql',
      sh: 'shell',
      bash: 'shell',
      json: 'json',
      xml: 'xml',
      html: 'html',
      css: 'css',
      scss: 'scss',
      sass: 'sass',
      md: 'markdown',
      yaml: 'yaml',
      yml: 'yaml',
      toml: 'toml',
      ini: 'ini',
      txt: 'plaintext',
    };
    return languageMap[ext] || 'plaintext';
  };

  return (
    <>
      <div className="editor-tabs">
        {file ? (
          <div className="editor-tab active">
            <span className="file-icon">📄</span>
            <span>{file.name}</span>
          </div>
        ) : (
          <div className="editor-tab active">
            <span>欢迎</span>
          </div>
        )}
      </div>
      <div className="editor-wrapper">
        {loading ? (
          <SkeletonEditor />
        ) : file ? (
          <MonacoEditor
            height="100%"
            language={getLanguageFromFilename(file.name)}
            theme="vs-dark"
            value={content}
            onChange={handleEditorChange}
            onMount={handleEditorDidMount}
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              lineNumbers: 'on',
              roundedSelection: false,
              scrollBeyondLastLine: false,
              automaticLayout: true,
              tabSize: 2,
              insertSpaces: true,
              wordWrap: 'on',
              formatOnPaste: true,
              formatOnType: true,
            }}
          />
        ) : filesError ? (
          <div className="editor-empty editor-empty-error">
            <div className="editor-empty-icon editor-empty-icon-error" aria-hidden>
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
            </div>
            <p className="editor-empty-title">{filesError}</p>
            <p className="editor-empty-hint">请确保后端服务已启动（如 8081 端口）</p>
            {onRetry && (
              <button type="button" className="editor-empty-retry" onClick={onRetry}>
                重试
              </button>
            )}
          </div>
        ) : (
          <div className="editor-empty">
            <div className="editor-empty-icon" aria-hidden>
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
                <line x1="16" y1="13" x2="8" y2="13" />
                <line x1="16" y1="17" x2="8" y2="17" />
                <polyline points="10 9 9 9 8 9" />
              </svg>
            </div>
            <p className="editor-empty-title">选择文件进行编辑</p>
            <p className="editor-empty-hint">从左侧资源管理器中点击一个文件，或使用「新建文件」创建新文件</p>
          </div>
        )}
      </div>
    </>
  );
});

export default Editor;
