import { FileInfo, MountInfo, HealthStatus, StreamReadResult, StreamWriteInput } from './types';

export interface EvifConfig {
  baseUrl?: string;
  timeout?: number;
  apiKey?: string;
}

export class EvifClient {
  private baseUrl: string;
  private timeout: number;
  private apiKey?: string;

  constructor(config?: EvifConfig | string) {
    if (typeof config === 'string') {
      this.baseUrl = config;
      this.timeout = 30000;
    } else {
      this.baseUrl = config?.baseUrl ?? 'http://localhost:8080';
      this.timeout = config?.timeout ?? 30000;
      this.apiKey = config?.apiKey;
    }
  }

  private async request(method: string, path: string, body?: unknown): Promise<any> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
      signal: AbortSignal.timeout(this.timeout),
    });

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new Error(`EVIF error ${response.status}: ${text}`);
    }

    return response.json();
  }

  // ===== File Operations =====

  async ls(path: string): Promise<FileInfo[]> {
    const result = await this.request('POST', '/api/v1/fs/ls', { path });
    if (Array.isArray(result)) return result;
    return result.entries ?? result;
  }

  async cat(path: string, offset?: number, size?: number): Promise<string> {
    const body: Record<string, unknown> = { path };
    if (offset !== undefined) body.offset = offset;
    if (size !== undefined) body.size = size;
    const result = await this.request('POST', '/api/v1/fs/read', body);
    return result.data ?? result.content ?? result;
  }

  async write(path: string, content: string, offset?: number): Promise<number> {
    const body: Record<string, unknown> = { path, data: content };
    if (offset !== undefined) body.offset = offset;
    const result = await this.request('POST', '/api/v1/fs/write', body);
    return result.bytes_written ?? result.size ?? result;
  }

  // ===== Streaming Operations =====

  async *streamRead(path: string): StreamReadResult {
    const headers: Record<string, string> = {};
    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const response = await fetch(`${this.baseUrl}/api/v1/fs/read`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...headers,
      },
      body: JSON.stringify({ path }),
      signal: AbortSignal.timeout(this.timeout),
    });

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new Error(`EVIF error ${response.status}: ${text}`);
    }

    if (!response.body) {
      throw new Error('EVIF streaming error: no response body');
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        if (chunk) yield chunk;
      }
    } finally {
      reader.releaseLock();
    }
  }

  async streamWrite(path: string, content: StreamWriteInput): Promise<number> {
    const headers: Record<string, string> = {};
    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    let totalBytes = 0;

    const stream = new ReadableStream({
      async start(controller) {
        const encoder = new TextEncoder();

        if (typeof content === 'string') {
          const encoded = encoder.encode(content);
          totalBytes = encoded.byteLength;
          controller.enqueue(encoded);
          controller.close();
        } else {
          for await (const chunk of content) {
            const encoded = encoder.encode(chunk);
            totalBytes += encoded.byteLength;
            controller.enqueue(encoded);
          }
          controller.close();
        }
      },
    });

    const response = await fetch(`${this.baseUrl}/api/v1/fs/write`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/octet-stream',
        'X-Path': path,
        ...headers,
      },
      body: stream,
      signal: AbortSignal.timeout(this.timeout),
    });

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new Error(`EVIF error ${response.status}: ${text}`);
    }

    const result = await response.json();
    return result.bytes_written ?? result.size ?? totalBytes;
  }

  async mkdir(path: string, mode?: number): Promise<boolean> {
    const body: Record<string, unknown> = { path };
    if (mode !== undefined) body.perm = mode;
    const result = await this.request('POST', '/api/v1/fs/mkdir', body);
    return result.success ?? result.ok ?? true;
  }

  async rm(path: string, recursive?: boolean): Promise<boolean> {
    const body: Record<string, unknown> = { path };
    if (recursive !== undefined) body.recursive = recursive;
    const result = await this.request('DELETE', '/api/v1/fs/remove', body);
    return result.success ?? result.ok ?? true;
  }

  async stat(path: string): Promise<FileInfo> {
    const result = await this.request('POST', '/api/v1/fs/stat', { path });
    return result;
  }

  async mv(oldPath: string, newPath: string): Promise<boolean> {
    const result = await this.request('POST', '/api/v1/fs/rename', {
      old_path: oldPath,
      new_path: newPath,
    });
    return result.success ?? result.ok ?? true;
  }

  async cp(src: string, dst: string): Promise<boolean> {
    const result = await this.request('POST', '/api/v1/fs/copy', {
      src_path: src,
      dst_path: dst,
    });
    return result.success ?? result.ok ?? true;
  }

  async grep(path: string, pattern: string, recursive?: boolean): Promise<string[]> {
    const body: Record<string, unknown> = { path, pattern };
    if (recursive !== undefined) body.recursive = recursive;
    const result = await this.request('POST', '/api/v1/fs/grep', body);
    return result.matches ?? result.lines ?? result;
  }

  async create(path: string): Promise<boolean> {
    const result = await this.request('POST', '/api/v1/fs/create', { path });
    return result.success ?? result.ok ?? true;
  }

  // ===== Mount Operations =====

  async mount(plugin: string, path: string, options?: Record<string, string>): Promise<boolean> {
    const body: Record<string, unknown> = { plugin, path };
    if (options) body.options = options;
    const result = await this.request('POST', '/api/v1/mount/add', body);
    return result.success ?? result.ok ?? true;
  }

  async unmount(path: string): Promise<boolean> {
    const result = await this.request('POST', '/api/v1/mount/remove', { path });
    return result.success ?? result.ok ?? true;
  }

  async mounts(): Promise<MountInfo[]> {
    const result = await this.request('GET', '/api/v1/mount/list');
    if (Array.isArray(result)) return result;
    return result.mounts ?? result;
  }

  // ===== Health =====

  async health(): Promise<HealthStatus> {
    const result = await this.request('GET', '/health');
    return result;
  }

  // ===== Context API =====

  async contextRead(path: string): Promise<string> {
    return this.cat(`/context/${path}`);
  }

  async contextWrite(path: string, content: string): Promise<number> {
    return this.write(`/context/${path}`, content);
  }

  async contextList(layer?: string): Promise<FileInfo[]> {
    return this.ls(layer ? `/context/${layer}` : '/context');
  }

  async contextCurrent(): Promise<string> {
    return this.contextRead('L0/current');
  }

  async contextUpdateCurrent(context: string): Promise<number> {
    return this.contextWrite('L0/current', context);
  }

  async contextDecisions(): Promise<string> {
    return this.contextRead('L1/decisions.md');
  }

  async contextAddDecision(decision: string): Promise<number> {
    const existing = await this.contextDecisions();
    const updated = existing + '\n' + decision;
    return this.contextWrite('L1/decisions.md', updated);
  }

  async contextRecentOps(): Promise<any[]> {
    const raw = await this.contextRead('L1/recent_ops.json');
    return JSON.parse(raw);
  }

  async contextSearch(query: string, layer?: string): Promise<string[]> {
    const basePath = layer ? `/context/${layer}` : '/context';
    const result = await this.grep(basePath, query, true);
    return result;
  }

  async contextMeta(): Promise<any> {
    const raw = await this.contextRead('L0/meta.json');
    return JSON.parse(raw);
  }

  async contextKnowledge(name: string): Promise<string> {
    return this.contextRead(`L2/${name}`);
  }

  async contextAddKnowledge(name: string, content: string): Promise<number> {
    return this.contextWrite(`L2/${name}`, content);
  }

  // ===== Skill API =====

  async skillDiscover(): Promise<string[]> {
    const entries = await this.ls('/skills');
    return entries.filter((e) => e.is_dir).map((e) => e.name);
  }

  async skillRead(name: string): Promise<string> {
    return this.cat(`/skills/${name}/SKILL.md`);
  }

  async skillExecute(name: string, input: string): Promise<string> {
    await this.write(`/skills/${name}/input`, input);
    return this.cat(`/skills/${name}/output`);
  }

  async skillRegister(name: string, skillMd: string): Promise<boolean> {
    const created = await this.mkdir(`/skills/${name}`);
    if (!created) return false;
    await this.write(`/skills/${name}/SKILL.md`, skillMd);
    return true;
  }

  async skillMatch(query: string): Promise<string | null> {
    const skills = await this.skillDiscover();
    for (const name of skills) {
      try {
        const content = await this.skillRead(name);
        const triggerMatch = content.match(/##\s*Triggers?\s*\n([\s\S]*?)(?=\n##|\n*$)/i);
        if (triggerMatch) {
          const triggers = triggerMatch[1]
            .split('\n')
            .map((line) => line.replace(/^[-*]\s*/, '').trim())
            .filter((line) => line.length > 0);
          for (const trigger of triggers) {
            if (query.toLowerCase().includes(trigger.toLowerCase())) {
              return name;
            }
          }
        }
      } catch {
        // Skip skills that cannot be read
        continue;
      }
    }
    return null;
  }

  async skillRemove(name: string): Promise<boolean> {
    return this.rm(`/skills/${name}`, true);
  }
}
