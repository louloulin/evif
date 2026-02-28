/**
 * 获取与管理认证令牌的工具函数
 * - 令牌存储在 localStorage 中，键名由环境变量 VITE_TOKEN_KEY 指定
 * - 提供获取、设置、清除令牌，以及构建带认证头的请求头
 */
export const TOKEN_KEY: string = (import.meta.env?.VITE_TOKEN_KEY as string) || 'evif_auth_token'

/**
 * 获取当前认证令牌
 */
export function getToken(): string | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage.getItem(TOKEN_KEY) : null
  } catch {
    return null
  }
}

/**
 * 设置认证令牌
 */
export function setToken(token: string): void {
  if (typeof window !== 'undefined') {
    window.localStorage.setItem(TOKEN_KEY, token)
  }
}

/**
 * 清除认证令牌
 */
export function clearToken(): void {
  if (typeof window !== 'undefined') {
    window.localStorage.removeItem(TOKEN_KEY)
  }
}

/**
 * 基于现有请求头构建附带 Authorization 的请求头
 */
export function buildAuthHeaders(base?: HeadersInit): HeadersInit {
  const token = getToken()
  const normalized: Record<string, string> = {}

  // 规范化基础头
  if (base) {
    if (Array.isArray(base)) {
      for (const [k, v] of base) normalized[k] = v
    } else if (base instanceof Headers) {
      base.forEach((v, k) => (normalized[k] = v))
    } else {
      Object.assign(normalized, base)
    }
  }

  // 添加认证头
  if (token && !normalized['Authorization']) {
    normalized['Authorization'] = `Bearer ${token}`
  }

  return normalized
}
