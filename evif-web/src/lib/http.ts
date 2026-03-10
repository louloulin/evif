import { buildAuthHeaders, clearToken } from '@/lib/auth'

// 默认配置
const DEFAULT_TIMEOUT = 30000 // 30秒
const DEFAULT_RETRIES = 3
const DEFAULT_RETRY_DELAY = 1000 // 1秒

/**
 * HTTP 请求配置选项
 */
export interface FetchOptions extends RequestInit {
  /** 请求超时时间（毫秒），默认 30000 */
  timeout?: number
  /** 最大重试次数，默认 3 */
  retries?: number
  /** 重试间隔（毫秒），默认 1000 */
  retryDelay?: number
  /** 是否在重试时显示日志 */
  verbose?: boolean
}

/**
 * 带自动附加认证头、超时处理与错误处理的 fetch 包装
 * - 自动在请求头加入 Authorization: Bearer <token>
 * - 捕获 401 并清除本地令牌，抛出明确错误
 * - 支持超时自动取消
 * - 支持自动重试机制
 */
export async function httpFetch(
  input: RequestInfo | URL,
  init?: FetchOptions
): Promise<Response> {
  const {
    timeout = DEFAULT_TIMEOUT,
    retries = DEFAULT_RETRIES,
    retryDelay = DEFAULT_RETRY_DELAY,
    verbose = false,
    ...fetchInit
  } = init || {}

  const headers = buildAuthHeaders(fetchInit?.headers)

  // 辅助函数：执行单次请求
  const executeRequest = async (): Promise<Response> => {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), timeout)

    try {
      const resp = await fetch(input, {
        ...fetchInit,
        headers,
        signal: controller.signal
      })

      clearTimeout(timeoutId)

      if (resp.status === 401) {
        clearToken()
        throw new Error('未认证或令牌无效')
      }

      return resp
    } catch (error) {
      clearTimeout(timeoutId)

      // 处理超时
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(`请求超时（${timeout}ms）`)
      }

      throw error
    }
  }

  // 辅助函数：带重试的执行
  const executeWithRetry = async (attempt: number): Promise<Response> => {
    try {
      return await executeRequest()
    } catch (error) {
      const isNetworkError = error instanceof TypeError &&
        error.message.includes('Failed to fetch')

      // 判断是否可重试
      const canRetry = attempt < retries &&
        (isNetworkError || error instanceof Error && (
          error.message.includes('network') ||
          error.message.includes('timeout') ||
          error.message.includes('timeout') ||
          error.message.includes('fetch')
        ))

      if (canRetry && attempt < retries) {
        if (verbose) {
          console.log(`[httpFetch] 重试 ${attempt + 1}/${retries}...`)
        }

        // 指数退避
        const delay = retryDelay * Math.pow(2, attempt)
        await new Promise(resolve => setTimeout(resolve, delay))

        return executeWithRetry(attempt + 1)
      }

      throw error
    }
  }

  return executeWithRetry(0)
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

/**
 * 创建可取消的 fetch 请求
 * 返回一个包含 abort 函数的对象
 */
export function createCancellableFetch(
  input: RequestInfo | URL,
  init?: FetchOptions
): { promise: Promise<Response>; abort: () => void } {
  const controller = new AbortController()
  const timeout = init?.timeout || DEFAULT_TIMEOUT
  const timeoutId = setTimeout(() => controller.abort(), timeout)

  const promise = httpFetch(input, {
    ...init,
    signal: controller.signal
  }).finally(() => {
    clearTimeout(timeoutId)
  })

  return {
    promise,
    abort: () => {
      clearTimeout(timeoutId)
      controller.abort()
    }
  }
}
