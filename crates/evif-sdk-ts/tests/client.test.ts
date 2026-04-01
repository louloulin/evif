import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EvifClient } from '../src/client';

// ----- Helpers -----

function mockFetchResponse(data: any, status = 200): void {
  (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    json: () => Promise.resolve(data),
    text: () => Promise.resolve(JSON.stringify(data)),
  });
}

function lastFetchCall(): { method: string; url: string; body: any } {
  const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
  const last = calls[calls.length - 1];
  return {
    method: last[1]?.method ?? 'GET',
    url: last[0],
    body: last[1]?.body ? JSON.parse(last[1].body) : undefined,
  };
}

// ----- Tests -----

describe('EvifClient', () => {
  let client: EvifClient;

  beforeEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = vi.fn();
    client = new EvifClient({ baseUrl: 'http://localhost:8080' });
  });

  // ===== Constructor =====

  describe('constructor', () => {
    it('uses default baseUrl when no config provided', () => {
      const c = new EvifClient();
      expect(c).toBeInstanceOf(EvifClient);
    });

    it('accepts a string as baseUrl', () => {
      const c = new EvifClient('http://remote:9090');
      expect(c).toBeInstanceOf(EvifClient);
    });

    it('accepts an EvifConfig object', () => {
      const c = new EvifClient({ baseUrl: 'http://remote:9090', timeout: 5000, apiKey: 'test-key' });
      expect(c).toBeInstanceOf(EvifClient);
    });
  });

  // ===== File Operations =====

  describe('file operations', () => {
    describe('ls', () => {
      it('calls POST /api/v1/fs/ls and returns entries', async () => {
        const entries = [
          { name: 'file.txt', path: '/file.txt', size: 100, mode: 0o644, mtime: 1000, is_dir: false, is_file: true },
        ];
        mockFetchResponse({ entries });

        const result = await client.ls('/');

        expect(result).toEqual(entries);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/ls');
        expect(call.body).toEqual({ path: '/' });
      });

      it('handles response without entries wrapper', async () => {
        const entries = [
          { name: 'a.txt', path: '/a.txt', size: 0, mode: 0, mtime: 0, is_dir: false, is_file: true },
        ];
        mockFetchResponse(entries);

        const result = await client.ls('/');
        expect(result).toEqual(entries);
      });
    });

    describe('cat', () => {
      it('calls POST /api/v1/fs/read and returns data', async () => {
        mockFetchResponse({ data: 'hello world' });

        const result = await client.cat('/file.txt');

        expect(result).toBe('hello world');
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/read');
        expect(call.body).toEqual({ path: '/file.txt' });
      });

      it('passes offset and size when provided', async () => {
        mockFetchResponse({ data: 'partial' });

        await client.cat('/file.txt', 10, 20);

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/file.txt', offset: 10, size: 20 });
      });
    });

    describe('write', () => {
      it('calls POST /api/v1/fs/write and returns bytes written', async () => {
        mockFetchResponse({ bytes_written: 11 });

        const result = await client.write('/file.txt', 'hello world');

        expect(result).toBe(11);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/write');
        expect(call.body).toEqual({ path: '/file.txt', data: 'hello world' });
      });

      it('passes offset when provided', async () => {
        mockFetchResponse({ bytes_written: 5 });

        await client.write('/file.txt', 'hello', 10);

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/file.txt', data: 'hello', offset: 10 });
      });
    });

    describe('mkdir', () => {
      it('calls POST /api/v1/fs/mkdir', async () => {
        mockFetchResponse({ success: true });

        const result = await client.mkdir('/newdir');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/mkdir');
        expect(call.body).toEqual({ path: '/newdir' });
      });

      it('passes mode (perm) when provided', async () => {
        mockFetchResponse({ success: true });

        await client.mkdir('/dir', 0o755);

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/dir', perm: 0o755 });
      });
    });

    describe('rm', () => {
      it('calls DELETE /api/v1/fs/remove', async () => {
        mockFetchResponse({ success: true });

        const result = await client.rm('/file.txt');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('DELETE');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/remove');
        expect(call.body).toEqual({ path: '/file.txt' });
      });

      it('passes recursive flag when provided', async () => {
        mockFetchResponse({ success: true });

        await client.rm('/dir', true);

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/dir', recursive: true });
      });
    });

    describe('stat', () => {
      it('calls POST /api/v1/fs/stat', async () => {
        const info = { name: 'file.txt', path: '/file.txt', size: 100, mode: 0o644, mtime: 1000, is_dir: false, is_file: true };
        mockFetchResponse(info);

        const result = await client.stat('/file.txt');

        expect(result).toEqual(info);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/stat');
        expect(call.body).toEqual({ path: '/file.txt' });
      });
    });

    describe('mv', () => {
      it('calls POST /api/v1/fs/rename', async () => {
        mockFetchResponse({ success: true });

        const result = await client.mv('/old.txt', '/new.txt');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/rename');
        expect(call.body).toEqual({ old_path: '/old.txt', new_path: '/new.txt' });
      });
    });

    describe('cp', () => {
      it('calls POST /api/v1/fs/copy', async () => {
        mockFetchResponse({ success: true });

        const result = await client.cp('/src.txt', '/dst.txt');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/copy');
        expect(call.body).toEqual({ src_path: '/src.txt', dst_path: '/dst.txt' });
      });
    });

    describe('grep', () => {
      it('calls POST /api/v1/fs/grep and returns matches', async () => {
        mockFetchResponse({ matches: ['line 1 match', 'line 5 match'] });

        const result = await client.grep('/dir', 'pattern');

        expect(result).toEqual(['line 1 match', 'line 5 match']);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/grep');
        expect(call.body).toEqual({ path: '/dir', pattern: 'pattern' });
      });

      it('passes recursive flag when provided', async () => {
        mockFetchResponse({ matches: [] });

        await client.grep('/dir', 'test', true);

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/dir', pattern: 'test', recursive: true });
      });
    });

    describe('create', () => {
      it('calls POST /api/v1/fs/create', async () => {
        mockFetchResponse({ success: true });

        const result = await client.create('/newfile.txt');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/fs/create');
        expect(call.body).toEqual({ path: '/newfile.txt' });
      });
    });
  });

  // ===== Mount Operations =====

  describe('mount operations', () => {
    describe('mount', () => {
      it('calls POST /api/v1/mount/add', async () => {
        mockFetchResponse({ success: true });

        const result = await client.mount('memory', '/mnt/data', { readOnly: 'true' });

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/mount/add');
        expect(call.body).toEqual({ plugin: 'memory', path: '/mnt/data', options: { readOnly: 'true' } });
      });

      it('omits options when not provided', async () => {
        mockFetchResponse({ success: true });

        await client.mount('memory', '/mnt/data');

        const call = lastFetchCall();
        expect(call.body).toEqual({ plugin: 'memory', path: '/mnt/data' });
      });
    });

    describe('unmount', () => {
      it('calls POST /api/v1/mount/remove', async () => {
        mockFetchResponse({ success: true });

        const result = await client.unmount('/mnt/data');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('POST');
        expect(call.url).toBe('http://localhost:8080/api/v1/mount/remove');
        expect(call.body).toEqual({ path: '/mnt/data' });
      });
    });

    describe('mounts', () => {
      it('calls GET /api/v1/mount/list', async () => {
        const mounts = [
          { path: '/mnt/a', plugin: 'memory', options: {} },
          { path: '/mnt/b', plugin: 'disk', options: { root: '/tmp' } },
        ];
        mockFetchResponse({ mounts });

        const result = await client.mounts();

        expect(result).toEqual(mounts);
        const call = lastFetchCall();
        expect(call.method).toBe('GET');
        expect(call.url).toBe('http://localhost:8080/api/v1/mount/list');
      });
    });
  });

  // ===== Health =====

  describe('health', () => {
    it('calls GET /health', async () => {
      const status = { status: 'ok', version: '0.1.0', uptime: 42, plugins_count: 2 };
      mockFetchResponse(status);

      const result = await client.health();

      expect(result).toEqual(status);
      const call = lastFetchCall();
      expect(call.method).toBe('GET');
      expect(call.url).toBe('http://localhost:8080/health');
    });
  });

  // ===== Context API =====

  describe('context API', () => {
    describe('contextRead', () => {
      it('reads from /context/<path>', async () => {
        mockFetchResponse({ data: 'context data' });

        const result = await client.contextRead('L0/current');

        expect(result).toBe('context data');
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L0/current' });
      });
    });

    describe('contextWrite', () => {
      it('writes to /context/<path>', async () => {
        mockFetchResponse({ bytes_written: 7 });

        const result = await client.contextWrite('L0/current', 'my task');

        expect(result).toBe(7);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L0/current', data: 'my task' });
      });
    });

    describe('contextList', () => {
      it('lists /context when no layer specified', async () => {
        const entries = [
          { name: 'L0', path: '/context/L0', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
        ];
        mockFetchResponse({ entries });

        const result = await client.contextList();

        expect(result).toEqual(entries);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context' });
      });

      it('lists /context/<layer> when layer specified', async () => {
        const entries = [
          { name: 'current', path: '/context/L0/current', size: 10, mode: 0, mtime: 0, is_dir: false, is_file: true },
        ];
        mockFetchResponse({ entries });

        const result = await client.contextList('L0');

        expect(result).toEqual(entries);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L0' });
      });
    });

    describe('contextCurrent', () => {
      it('reads L0/current', async () => {
        mockFetchResponse({ data: 'current task' });

        const result = await client.contextCurrent();

        expect(result).toBe('current task');
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L0/current' });
      });
    });

    describe('contextUpdateCurrent', () => {
      it('writes to L0/current', async () => {
        mockFetchResponse({ bytes_written: 8 });

        const result = await client.contextUpdateCurrent('new task');

        expect(result).toBe(8);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L0/current', data: 'new task' });
      });
    });

    describe('contextDecisions', () => {
      it('reads L1/decisions.md', async () => {
        mockFetchResponse({ data: '- Use TypeScript\n' });

        const result = await client.contextDecisions();

        expect(result).toBe('- Use TypeScript\n');
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L1/decisions.md' });
      });
    });

    describe('contextAddDecision', () => {
      it('appends a decision to L1/decisions.md', async () => {
        // First call: read existing decisions
        mockFetchResponse({ data: '- Decision A\n' });
        // Second call: write updated decisions
        mockFetchResponse({ bytes_written: 28 });

        const result = await client.contextAddDecision('- Decision B');

        expect(result).toBe(28);

        // Verify write call
        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
        const writeCall = calls[1];
        const writeBody = JSON.parse(writeCall[1].body);
        expect(writeBody).toEqual({
          path: '/context/L1/decisions.md',
          data: '- Decision A\n\n- Decision B',
        });
      });
    });

    describe('contextRecentOps', () => {
      it('reads and parses L1/recent_ops.json', async () => {
        const ops = [{ op: 'write', path: '/foo' }, { op: 'read', path: '/bar' }];
        mockFetchResponse({ data: JSON.stringify(ops) });

        const result = await client.contextRecentOps();

        expect(result).toEqual(ops);
      });
    });

    describe('contextSearch', () => {
      it('greps /context with query', async () => {
        mockFetchResponse({ matches: ['found line 1', 'found line 2'] });

        const result = await client.contextSearch('test query');

        expect(result).toEqual(['found line 1', 'found line 2']);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context', pattern: 'test query', recursive: true });
      });

      it('greps within a specific layer', async () => {
        mockFetchResponse({ matches: ['match'] });

        await client.contextSearch('test', 'L1');

        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L1', pattern: 'test', recursive: true });
      });
    });

    describe('contextMeta', () => {
      it('reads and parses L0/meta.json', async () => {
        const meta = { sessionId: 'abc-123', createdAt: '2025-01-01' };
        mockFetchResponse({ data: JSON.stringify(meta) });

        const result = await client.contextMeta();

        expect(result).toEqual(meta);
      });
    });

    describe('contextKnowledge', () => {
      it('reads from L2/<name>', async () => {
        mockFetchResponse({ data: 'knowledge content' });

        const result = await client.contextKnowledge('typescript-patterns');

        expect(result).toBe('knowledge content');
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L2/typescript-patterns' });
      });
    });

    describe('contextAddKnowledge', () => {
      it('writes to L2/<name>', async () => {
        mockFetchResponse({ bytes_written: 18 });

        const result = await client.contextAddKnowledge('rust-tips', 'use cargo clippy');

        expect(result).toBe(18);
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/context/L2/rust-tips', data: 'use cargo clippy' });
      });
    });
  });

  // ===== Skill API =====

  describe('skill API', () => {
    describe('skillDiscover', () => {
      it('lists directories under /skills', async () => {
        const entries = [
          { name: 'summarize', path: '/skills/summarize', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
          { name: 'translate', path: '/skills/translate', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
          { name: 'SKILL.md', path: '/skills/SKILL.md', size: 100, mode: 0, mtime: 0, is_dir: false, is_file: true },
        ];
        mockFetchResponse({ entries });

        const result = await client.skillDiscover();

        expect(result).toEqual(['summarize', 'translate']);
      });
    });

    describe('skillRead', () => {
      it('reads SKILL.md for a named skill', async () => {
        mockFetchResponse({ data: '# Summarize Skill\nDescription here' });

        const result = await client.skillRead('summarize');

        expect(result).toBe('# Summarize Skill\nDescription here');
        const call = lastFetchCall();
        expect(call.body).toEqual({ path: '/skills/summarize/SKILL.md' });
      });
    });

    describe('skillExecute', () => {
      it('writes input then reads output', async () => {
        // First call: write input
        mockFetchResponse({ bytes_written: 12 });
        // Second call: read output
        mockFetchResponse({ data: 'summary result' });

        const result = await client.skillExecute('summarize', 'input text!');

        expect(result).toBe('summary result');

        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
        const writeCall = calls[0];
        const readCall = calls[1];

        const writeBody = JSON.parse(writeCall[1].body);
        expect(writeBody).toEqual({ path: '/skills/summarize/input', data: 'input text!' });

        const readBody = JSON.parse(readCall[1].body);
        expect(readBody).toEqual({ path: '/skills/summarize/output' });
      });
    });

    describe('skillRegister', () => {
      it('creates directory and writes SKILL.md', async () => {
        // First call: mkdir
        mockFetchResponse({ success: true });
        // Second call: write SKILL.md
        mockFetchResponse({ bytes_written: 20 });

        const result = await client.skillRegister('my-skill', '# My Skill\nDoes things');

        expect(result).toBe(true);

        const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
        const mkdirCall = calls[0];
        const writeCall = calls[1];

        const mkdirBody = JSON.parse(mkdirCall[1].body);
        expect(mkdirBody).toEqual({ path: '/skills/my-skill' });

        const writeBody = JSON.parse(writeCall[1].body);
        expect(writeBody).toEqual({ path: '/skills/my-skill/SKILL.md', data: '# My Skill\nDoes things' });
      });

      it('returns false if mkdir fails', async () => {
        mockFetchResponse({ success: false });

        const result = await client.skillRegister('bad-skill', 'content');

        expect(result).toBe(false);
      });
    });

    describe('skillMatch', () => {
      it('matches a skill based on triggers', async () => {
        // skillDiscover -> ls
        mockFetchResponse({
          entries: [
            { name: 'summarize', path: '/skills/summarize', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
          ],
        });
        // skillRead -> cat
        mockFetchResponse({
          data: '# Summarize\n\n## Triggers\n- summarize\n- tldr\n- summary\n\n## Description\nSummarizes text.',
        });

        const result = await client.skillMatch('please summarize this');

        expect(result).toBe('summarize');
      });

      it('returns null when no triggers match', async () => {
        mockFetchResponse({
          entries: [
            { name: 'summarize', path: '/skills/summarize', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
          ],
        });
        mockFetchResponse({
          data: '# Summarize\n\n## Triggers\n- summarize\n- tldr\n',
        });

        const result = await client.skillMatch('translate this to french');

        expect(result).toBeNull();
      });

      it('returns null when no skills exist', async () => {
        mockFetchResponse({ entries: [] });

        const result = await client.skillMatch('something');

        expect(result).toBeNull();
      });

      it('skips skills that fail to read', async () => {
        // skillDiscover
        mockFetchResponse({
          entries: [
            { name: 'broken', path: '/skills/broken', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
            { name: 'working', path: '/skills/working', size: 0, mode: 0, mtime: 0, is_dir: true, is_file: false },
          ],
        });
        // broken skill read -> error
        (globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('read failed'));
        // working skill read
        mockFetchResponse({
          data: '# Working\n\n## Triggers\n- work\n- do work\n',
        });

        const result = await client.skillMatch('do work now');

        expect(result).toBe('working');
      });
    });

    describe('skillRemove', () => {
      it('removes skill directory recursively', async () => {
        mockFetchResponse({ success: true });

        const result = await client.skillRemove('old-skill');

        expect(result).toBe(true);
        const call = lastFetchCall();
        expect(call.method).toBe('DELETE');
        expect(call.body).toEqual({ path: '/skills/old-skill', recursive: true });
      });
    });
  });

  // ===== Error Handling =====

  describe('error handling', () => {
    it('throws on non-2xx response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 404,
        text: () => Promise.resolve('Not Found'),
        json: () => Promise.resolve({}),
      });

      await expect(client.ls('/nonexistent')).rejects.toThrow('EVIF error 404: Not Found');
    });

    it('handles text() failure gracefully in error', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.reject(new Error('text failed')),
        json: () => Promise.resolve({}),
      });

      await expect(client.ls('/crash')).rejects.toThrow('EVIF error 500: ');
    });
  });

  // ===== Auth =====

  describe('authentication', () => {
    it('sends Authorization header when apiKey is configured', async () => {
      const authClient = new EvifClient({ baseUrl: 'http://localhost:8080', apiKey: 'secret-key' });
      mockFetchResponse({ entries: [] });

      await authClient.ls('/');

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['Authorization']).toBe('Bearer secret-key');
    });

    it('does not send Authorization header without apiKey', async () => {
      mockFetchResponse({ entries: [] });

      await client.ls('/');

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['Authorization']).toBeUndefined();
    });
  });
});
