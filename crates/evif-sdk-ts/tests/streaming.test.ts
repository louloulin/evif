import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EvifClient } from '../src/client';

function lastFetchCall(): { method: string; url: string; body: any } {
  const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
  const last = calls[calls.length - 1];
  const rawBody = last[1]?.body;
  let parsedBody: any;
  if (rawBody && typeof rawBody === 'object' && 'getReader' in rawBody) {
    // It's a ReadableStream - can't easily parse without consuming it
    parsedBody = '[ReadableStream]';
  } else {
    try {
      parsedBody = rawBody ? JSON.parse(rawBody) : undefined;
    } catch {
      parsedBody = rawBody;
    }
  }
  return {
    method: last[1]?.method ?? 'GET',
    url: last[0],
    body: parsedBody,
  };
}

describe('EvifClient Streaming', () => {
  let client: EvifClient;

  beforeEach(() => {
    vi.restoreAllMocks();
    globalThis.fetch = vi.fn();
    client = new EvifClient({ baseUrl: 'http://localhost:8080' });
  });

  // ===== streamRead =====

  describe('streamRead', () => {
    it('is an AsyncGenerator function', async () => {
      // Create a mock streaming response
      const mockStream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('chunk1'));
          controller.enqueue(new TextEncoder().encode('chunk2'));
          controller.close();
        },
      });

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        body: mockStream,
      });

      const generator = client.streamRead('/large-file.txt');
      expect(generator[Symbol.asyncIterator]).toBeDefined();
    });

    it('yields chunks from streaming response', async () => {
      const chunks = ['Hello ', 'World!', ' Done'];
      const mockStream = new ReadableStream({
        start(controller) {
          for (const chunk of chunks) {
            controller.enqueue(new TextEncoder().encode(chunk));
          }
          controller.close();
        },
      });

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        body: mockStream,
      });

      const result: string[] = [];
      for await (const chunk of client.streamRead('/large-file.txt')) {
        result.push(chunk);
      }

      expect(result).toEqual(chunks);
    });

    it('sends POST request to /api/v1/fs/read with path', async () => {
      const mockStream = new ReadableStream({
        start(controller) {
          controller.close();
        },
      });

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        body: mockStream,
      });

      for await (const _ of client.streamRead('/data/file.bin')) {
        // consume the generator
      }

      const call = lastFetchCall();
      expect(call.method).toBe('POST');
      expect(call.url).toBe('http://localhost:8080/api/v1/fs/read');
      expect(call.body).toEqual({ path: '/data/file.bin' });
    });

    it('throws on non-ok response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve('Internal Server Error'),
      });

      const iterator = client.streamRead('/error-file');
      await expect(iterator.next()).rejects.toThrow('EVIF error 500: Internal Server Error');
    });

    it('throws when response body is null', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        body: null,
      });

      const iterator = client.streamRead('/empty');
      await expect(iterator.next()).rejects.toThrow('EVIF streaming error: no response body');
    });

    it('includes Authorization header when apiKey is set', async () => {
      const authClient = new EvifClient({
        baseUrl: 'http://localhost:8080',
        apiKey: 'test-secret',
      });

      const mockStream = new ReadableStream({
        start(controller) {
          controller.close();
        },
      });

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        body: mockStream,
      });

      for await (const _ of authClient.streamRead('/auth-file')) {
        // consume
      }

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['Authorization']).toBe('Bearer test-secret');
    });
  });

  // ===== streamWrite =====

  describe('streamWrite', () => {
    it('accepts a string as content', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 11 }),
        text: () => Promise.resolve(JSON.stringify({ bytes_written: 11 })),
      });

      const result = await client.streamWrite('/output.txt', 'hello world');

      expect(result).toBe(11);
      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      expect(calls[0][1].method).toBe('POST');
      expect(calls[0][0]).toBe('http://localhost:8080/api/v1/fs/write');
    });

    it('accepts an AsyncIterable as content', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 15 }),
      });

      async function* generateChunks(): AsyncGenerator<string> {
        yield 'chunk1';
        yield 'chunk2';
        yield 'chunk3';
      }

      const result = await client.streamWrite('/output.bin', generateChunks());

      expect(result).toBe(15);
    });

    it('sends X-Path header with the file path', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 5 }),
      });

      await client.streamWrite('/path/to/file.txt', 'hello');

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['X-Path']).toBe('/path/to/file.txt');
    });

    it('sends Content-Type as application/octet-stream', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 5 }),
      });

      await client.streamWrite('/file.txt', 'test');

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['Content-Type']).toBe('application/octet-stream');
    });

    it('throws on non-ok response', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve('Write failed'),
      });

      await expect(client.streamWrite('/error', 'data')).rejects.toThrow('EVIF error 500: Write failed');
    });

    it('includes Authorization header when apiKey is set', async () => {
      const authClient = new EvifClient({
        baseUrl: 'http://localhost:8080',
        apiKey: 'secret-key',
      });

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 5 }),
      });

      await authClient.streamWrite('/secure-file', 'content');

      const calls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls;
      const headers = calls[0][1].headers;
      expect(headers['Authorization']).toBe('Bearer secret-key');
    });

    it('returns bytes_written from response when available', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 42 }),
      });

      const result = await client.streamWrite('/file.txt', 'test');

      expect(result).toBe(42);
    });

    it('falls back to totalBytes when response has no bytes_written', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ success: true }),
      });

      const result = await client.streamWrite('/file.txt', 'hello world');

      expect(result).toBe(11);
    });

    it('handles empty string content', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 0 }),
      });

      const result = await client.streamWrite('/empty.txt', '');

      expect(result).toBe(0);
    });

    it('handles empty AsyncIterable content', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ bytes_written: 0 }),
      });

      async function* emptyGen(): AsyncGenerator<string> {
        // no yields
      }

      const result = await client.streamWrite('/empty.bin', emptyGen());

      expect(result).toBe(0);
    });

    it('streams content chunks from AsyncIterable', async () => {
      let sentChunks: string[] = [];

      (globalThis.fetch as ReturnType<typeof vi.fn>).mockImplementation(
        async (url: string, options: RequestInit) => {
          // Read the stream to verify chunks were sent
          const reader = (options.body as ReadableStream).getReader();
          const decoder = new TextDecoder();
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            sentChunks.push(decoder.decode(value));
          }
          return {
            ok: true,
            status: 200,
            json: () => Promise.resolve({ bytes_written: sentChunks.join('').length }),
          };
        }
      );

      async function* generateData(): AsyncGenerator<string> {
        yield 'first ';
        yield 'second ';
        yield 'third';
      }

      const result = await client.streamWrite('/streamed.txt', generateData());

      expect(sentChunks).toEqual(['first ', 'second ', 'third']);
      expect(result).toBe(18);
    });
  });
});
