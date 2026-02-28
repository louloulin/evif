/**
 * Phase 9.4: 搜索 API 服务
 * 对接 POST /api/v1/grep
 */

import { httpFetch } from '@/lib/http'
import type { SearchResponse, SearchResult } from '@/types/search'

interface GrepMatch {
  path: string
  line: number
  content: string
}

interface GrepApiResponse {
  pattern: string
  matches: GrepMatch[]
}

export async function searchGrep(path: string, pattern: string, recursive?: boolean): Promise<SearchResponse> {
  const res = await httpFetch('/api/v1/grep', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, pattern, recursive: recursive ?? true }),
  })
  if (!res.ok) {
    const t = await res.text()
    throw new Error(t || 'Search failed')
  }
  const data: GrepApiResponse = await res.json()
  const results: SearchResult[] = (data.matches || []).map((m) => ({
    path: m.path,
    line_number: m.line,
    line: m.content,
    preview: m.content,
  }))
  return {
    path,
    pattern: data.pattern,
    matches: results.length,
    results,
  }
}
