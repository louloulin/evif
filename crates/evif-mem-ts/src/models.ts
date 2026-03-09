/**
 * Data models for EVIF Memory API.
 */

export enum MemoryType {
  PROFILE = 'profile',
  EVENT = 'event',
  KNOWLEDGE = 'knowledge',
  BEHAVIOR = 'behavior',
  SKILL = 'skill',
  TOOL = 'tool',
  CONVERSATION = 'conversation',
  DOCUMENT = 'document',
}

export enum Modality {
  TEXT = 'text',
  CONVERSATION = 'conversation',
  DOCUMENT = 'document',
  IMAGE = 'image',
  VIDEO = 'video',
  AUDIO = 'audio',
}

export enum GraphQueryType {
  CAUSAL_CHAIN = 'causal_chain',
  TIMELINE = 'timeline',
  TEMPORAL_BFS = 'temporal_bfs',
  TEMPORAL_PATH = 'temporal_path',
}

export interface MemoryCreate {
  content: string;
  memoryType?: MemoryType | string;
  tags?: string[];
  modality?: Modality | string;
  references?: string[];
  metadata?: Record<string, unknown>;
}

export interface Memory {
  id: string;
  content: string;
  summary: string;
  memory_type: MemoryType | string;
  tags: string[];
  embedding?: number[];
  reinforcement_count: number;
  last_reinforced_at?: string;
  created_at: string;
  updated_at: string;
  references: string[];
  user_id?: string;
  tenant_id?: string;
  metadata: Record<string, unknown>;
}

export interface MemorySearchResult {
  memory: Memory;
  score: number;
}

export interface Category {
  id: string;
  name: string;
  description: string;
  summary: string;
  item_count: number;
  embedding?: number[];
  created_at: string;
  updated_at: string;
}

export interface GraphQuery {
  query: string;
  query_type: GraphQueryType | string;
  node_id?: string;
  max_depth?: number;
  limit?: number;
}

export interface GraphNode {
  id: string;
  node_type: string;
  label: string;
  metadata: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  edge_type: string;
  metadata: Record<string, unknown>;
}

export interface GraphResult {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: Record<string, unknown>;
}

export interface SearchOptions {
  k?: number;
  threshold?: number;
  mode?: 'vector' | 'hybrid' | 'rag';
}

export interface ListMemoriesOptions {
  limit?: number;
  offset?: number;
}

export interface GraphQueryOptions {
  queryType?: GraphQueryType | string;
  nodeId?: string;
  maxDepth?: number;
  limit?: number;
}
