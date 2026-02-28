import React, { useState } from 'react';
import { SkeletonFileTree } from './ui/skeleton';

interface FileNode {
  path: string;
  name: string;
  is_dir: boolean;
  children?: FileNode[];
}

interface FileTreeProps {
  files: FileNode[];
  getChildren: (path: string) => FileNode[];
  onExpandFolder: (path: string) => void;
  selectedFile: FileNode | null;
  onFileSelect: (file: FileNode) => void;
  onContextMenu: (e: React.MouseEvent, file: FileNode) => void;
  onContextMenuEmpty: (e: React.MouseEvent) => void;
  loading?: boolean;
  /** 正在加载子项的目录 path 集合，用于展开时显示「加载中…」 */
  loadingPaths?: Set<string>;
  error?: string | null;
  onRetry?: () => void;
}

const FileTree: React.FC<FileTreeProps> = ({
  files,
  getChildren,
  onExpandFolder,
  selectedFile,
  onFileSelect,
  onContextMenu,
  onContextMenuEmpty,
  loading = false,
  loadingPaths,
  error = null,
  onRetry
}) => {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['/']));

  const toggleFolder = (path: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
      onExpandFolder(path);
    }
    setExpandedFolders(newExpanded);
  };

  const getFileIcon = (node: FileNode) => {
    if (node.is_dir) {
      return expandedFolders.has(node.path) ? '📂' : '📁';
    }
    // File icons based on extension
    const ext = node.name.split('.').pop()?.toLowerCase() || '';
    const iconMap: Record<string, string> = {
      js: '📜',
      jsx: '⚛️',
      ts: '📘',
      tsx: '⚛️',
      py: '🐍',
      rs: '🦀',
      json: '📋',
      md: '📝',
      txt: '📄',
      html: '🌐',
      css: '🎨',
    };
    return iconMap[ext] || '📄';
  };

  const renderNode = (node: FileNode, level: number = 0): React.ReactNode => {
    const isExpanded = expandedFolders.has(node.path);
    const isSelected = selectedFile?.path === node.path;

    return (
      <div key={node.path}>
        <div
          className={`file-tree-item ${isSelected ? 'selected' : ''}`}
          style={{ paddingLeft: `${level * 16 + 12}px` }}
          onClick={() => {
            if (node.is_dir) {
              toggleFolder(node.path);
            } else {
              onFileSelect(node);
            }
          }}
          onContextMenu={(e) => onContextMenu(e, node)}
          role="button"
          tabIndex={0}
          aria-selected={isSelected}
        >
          {node.is_dir && (
            <span className={`folder-icon ${isExpanded ? 'open' : ''}`} aria-hidden="true">
              ▶
            </span>
          )}
          <span className="file-icon" aria-hidden="true">{getFileIcon(node)}</span>
          <span className="file-name">{node.name}</span>
        </div>
        {node.is_dir && isExpanded && (() => {
          const children = getChildren(node.path);
          if (children.length > 0) {
            return (
              <div>
                {children.map((child) => renderNode(child, level + 1))}
              </div>
            );
          }
          if (loadingPaths?.has(node.path)) {
            return (
              <div className="file-tree-loading" style={{ paddingLeft: `${(level + 1) * 16 + 12}px` }}>
                <span className="file-tree-empty-text">加载中…</span>
              </div>
            );
          }
          return null;
        })()}
      </div>
    );
  };

  return (
    <div className="file-tree">
      <div className="file-tree-header">EXPLORER</div>
      <div
        className="file-tree-list"
        onContextMenu={(e) => {
          if ((e.target as HTMLElement).closest('.file-tree-item')) return;
          onContextMenuEmpty(e);
        }}
      >
        {loading && (
          <div className="p-2 md:p-3">
            <SkeletonFileTree count={8} />
          </div>
        )}
        {!loading && error && (
          <div className="file-tree-empty file-tree-error">
            <span className="file-tree-empty-text">{error}</span>
            <span className="file-tree-empty-hint">请确保后端服务已启动（如 8081 端口）</span>
            {onRetry && (
              <button type="button" className="file-tree-retry min-h-[44px] min-w-[44px]" onClick={onRetry}>
                重试
              </button>
            )}
          </div>
        )}
        {!loading && !error && files.length === 0 && (
          <div className="file-tree-empty">
            <span className="file-tree-empty-text">暂无文件</span>
            <span className="file-tree-empty-hint">点击顶部「刷新」或检查后端</span>
          </div>
        )}
        {!loading && !error && files.length > 0 && files.map((file) => renderNode(file))}
      </div>
    </div>
  );
};

export default FileTree;
