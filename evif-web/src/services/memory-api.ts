/**
 * Memory API 服务
 * 对接 /api/v1/memories 和 /api/v1/categories 端点
 */

import { httpFetch } from '@/lib/http'

// 类型定义
export interface MemoryItem {
  id: string
  type: string
  content: string
  summary: string
  created: string
  updated: string
  category?: string
}

export interface Category {
  id: string
  name: string
  description: string
  item_count: number
  created: string
  updated: string
}

export interface CategoryWithMemories {
  category: Category
  memories: MemoryItem[]
}

export interface SearchResult {
  id: string
  type: string
  content: string
  score: number
  category?: string
}

export interface SearchResponse {
  results: SearchResult[]
  total: number
}

export interface GraphNode {
  id: string
  type: string
  label: string
  timestamp?: string
}

export interface TimelineEvent {
  node_id: string
  timestamp: string
  event_type: string
}

export interface GraphQueryResponse {
  query_type: string
  nodes?: GraphNode[]
  timeline?: TimelineEvent[]
  total: number
}

// API 函数
export async function listMemories(): Promise<MemoryItem[]> {
  const res = await httpFetch('/api/v1/memories')
  if (!res.ok) {
    throw new Error('Failed to fetch memories')
  }
  return res.json()
}

export async function getMemory(id: string): Promise<MemoryItem> {
  const res = await httpFetch(`/api/v1/memories/${id}`)
  if (!res.ok) {
    throw new Error(`Failed to fetch memory: ${id}`)
  }
  return res.json()
}

export async function createMemory(content: string, modality: string = 'text'): Promise<{ memory_id: string }> {
  const res = await httpFetch('/api/v1/memories', {
    method: 'POST',
    body: JSON.stringify({ content, modality }),
  })
  if (!res.ok) {
    throw new Error('Failed to create memory')
  }
  return res.json()
}

export async function searchMemories(query: string, mode: string = 'vector', vectorK: number = 10): Promise<SearchResponse> {
  const res = await httpFetch('/api/v1/memories/search', {
    method: 'POST',
    body: JSON.stringify({ query, mode, vector_k: vectorK }),
  })
  if (!res.ok) {
    throw new Error('Failed to search memories')
  }
  return res.json()
}

export async function listCategories(): Promise<Category[]> {
  const res = await httpFetch('/api/v1/categories')
  if (!res.ok) {
    throw new Error('Failed to fetch categories')
  }
  return res.json()
}

export async function getCategory(id: string): Promise<Category> {
  const res = await httpFetch(`/api/v1/categories/${id}`)
  if (!res.ok) {
    throw new Error(`Failed to fetch category: ${id}`)
  }
  return res.json()
}

export async function getCategoryMemories(id: string): Promise<CategoryWithMemories> {
  const res = await httpFetch(`/api/v1/categories/${id}/memories`)
  if (!res.ok) {
    throw new Error(`Failed to fetch category memories: ${id}`)
  }
  return res.json()
}

export async function queryGraph(
  queryType: string,
  options?: {
    startNode?: string
    endNode?: string
    maxDepth?: number
    eventType?: string
    category?: string
    startTime?: string
    endTime?: string
  }
): Promise<GraphQueryResponse> {
  const res = await httpFetch('/api/v1/graph/query', {
    method: 'POST',
    body: JSON.stringify({
      query_type: queryType,
      start_node: options?.startNode,
      end_node: options?.endNode,
      max_depth: options?.maxDepth ?? 5,
      event_type: options?.eventType,
      category: options?.category,
      start_time: options?.startTime,
      end_time: options?.endTime,
    }),
  })
  if (!res.ok) {
    throw new Error('Failed to query graph')
  }
  return res.json()
}
