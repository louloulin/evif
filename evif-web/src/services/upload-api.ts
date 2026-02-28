/**
 * Phase 9.4: 上传 API 服务
 * 对接 POST /api/v1/fs/write（文本）或 PUT /api/v1/files（base64，支持二进制）
 */

import { httpFetch } from '@/lib/http'

function arrayBufferToBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

/**
 * 上传单个文件到指定路径
 * @param file 本地文件
 * @param targetPath 目标路径（如 /mem/uploaded.txt）
 * @param onProgress 进度回调 (progress: 0-100)
 */
export async function uploadFile(
  file: File,
  targetPath: string,
  onProgress?: (progress: number) => void
): Promise<void> {
  const path = targetPath.endsWith('/') ? targetPath + file.name : targetPath.replace(/\/?$/, '/') + file.name
  const isLikelyText = file.type.startsWith('text/') || /\.(txt|md|json|js|ts|tsx|jsx|css|html|xml|yaml|yml|log)$/i.test(file.name)

  if (isLikelyText) {
    // 文本文件使用fetch(小文件通常不需要进度)
    if (onProgress) {
      onProgress(50) // 模拟进度
    }

    const content = await file.text()
    await httpFetch(`/api/v1/fs/create`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    }).catch(() => null)

    const res = await httpFetch(`/api/v1/fs/write?path=${encodeURIComponent(path)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content }),
    })

    if (onProgress) {
      onProgress(100)
    }

    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      throw new Error((data.message || data.error) || 'Upload failed')
    }
  } else {
    // 二进制文件使用XMLHttpRequest支持进度
    await new Promise<void>((resolve, reject) => {
      const xhr = new XMLHttpRequest()

      // 监听上传进度
      if (onProgress) {
        xhr.upload.addEventListener('progress', (e) => {
          if (e.lengthComputable) {
            const percentComplete = Math.round((e.loaded / e.total) * 100)
            onProgress(percentComplete)
          }
        })
      }

      xhr.addEventListener('load', () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve()
        } else {
          let errorMessage = 'Upload failed'
          try {
            const data = JSON.parse(xhr.responseText)
            errorMessage = data.message || data.error || errorMessage
          } catch {
            // ignore JSON parse error
          }
          reject(new Error(errorMessage))
        }
      })

      xhr.addEventListener('error', () => {
        reject(new Error('Network error'))
      })

      xhr.addEventListener('abort', () => {
        reject(new Error('Upload aborted'))
      })

      // 首先创建文件
      httpFetch(`/api/v1/files`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path }),
      }).catch(() => null)

      // 读取文件并上传
      file.arrayBuffer().then((buf) => {
        const data = arrayBufferToBase64(buf)
        xhr.open('PUT', `/api/v1/files?path=${encodeURIComponent(path)}`)
        xhr.setRequestHeader('Content-Type', 'application/json')
        xhr.send(JSON.stringify({ data, encoding: 'base64' }))
      }).catch(reject)
    })
  }
}

export interface UploadResult {
  success: boolean
  error?: string
}

/**
 * 上传多个文件到目录路径，返回与 files 同序的每项结果
 * @param onFileProgress 单个文件进度回调 (fileName: string, progress: 0-100)
 */
export async function uploadFiles(
  files: File[],
  directoryPath: string,
  onFileProgress?: (fileName: string, progress: number) => void
): Promise<{ success: number; failed: number; results: UploadResult[] }> {
  const base = directoryPath.replace(/\/?$/, '/')
  const results: UploadResult[] = []
  for (const file of files) {
    try {
      await uploadFile(file, base, (progress) => {
        if (onFileProgress) {
          onFileProgress(file.name, progress)
        }
      })
      results.push({ success: true })
    } catch (e) {
      results.push({ success: false, error: e instanceof Error ? e.message : 'Unknown error' })
    }
  }
  const success = results.filter((r) => r.success).length
  return { success, failed: files.length - success, results }
}
