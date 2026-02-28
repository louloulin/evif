import { buildAuthHeaders, clearToken } from '@/lib/auth'

/**
 * 带自动附加认证头与错误处理的 fetch 包装
 * - 自动在请求头加入 Authorization: Bearer <token>
 * - 捕获 401 并清除本地令牌，抛出明确错误
 */
export async function httpFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const headers = buildAuthHeaders(init?.headers)
  const resp = await fetch(input, { ...init, headers })

  if (resp.status === 401) {
    clearToken()
    throw new Error('未认证或令牌无效')
  }

  return resp
}

/**
 * 从错误响应 body 解析后端返回的 message，用于展示给用户
 */
export async function parseErrorResponse(response: Response, fallback: string): Promise<string> {
  try {
    const data = await response.json()
    if (data && typeof data === 'object') {
      if (typeof data.message === 'string' && data.message) return data.message
      if (typeof data.error === 'string' && data.error) return data.error
    }
  } catch {
    /* ignore */
  }
  return fallback
}
