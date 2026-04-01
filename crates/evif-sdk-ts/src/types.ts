export interface FileInfo {
  name: string;
  path: string;
  size: number;
  mode: number;
  mtime: number;
  is_dir: boolean;
  is_file: boolean;
}

export interface MountInfo {
  path: string;
  plugin: string;
  options: Record<string, string>;
}

export interface HealthStatus {
  status: string;
  version: string;
  uptime: number;
  plugins_count: number;
}

export type ContextLayer = 'L0' | 'L1' | 'L2';

export interface SearchResult {
  path: string;
  snippet: string;
  layer: ContextLayer;
}

export interface SkillInfo {
  name: string;
  description: string;
  triggers: string[];
}

// Streaming types
export type StreamReadResult = AsyncGenerator<string>;
export type StreamWriteInput = string | AsyncIterable<string>;
