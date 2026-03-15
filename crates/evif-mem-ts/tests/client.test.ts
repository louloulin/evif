/**
 * Unit tests for EvifMemoryClient
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import axios from 'axios';
import { MemoryConfig } from '../src/config';
import { MemoryType, Modality, GraphQueryType } from '../src/models';

const mockRequest = vi.fn();

vi.mock('axios', () => ({
  default: {
    create: vi.fn(() => ({
      request: mockRequest,
    })),
  },
}));

beforeEach(() => {
  mockRequest.mockReset();
  vi.clearAllMocks();
});

// Simple validation tests without complex mocking
describe('MemoryConfig', () => {
  it('should create config with default values', () => {
    const config = new MemoryConfig({ apiUrl: 'http://localhost:8080' });
    expect(config.apiUrl).toBe('http://localhost:8080');
    expect(config.timeout).toBe(30000);
    expect(config.maxRetries).toBe(3);
    expect(config.apiKey).toBeUndefined();
  });

  it('should create config with custom values', () => {
    const config = new MemoryConfig({
      apiUrl: 'http://localhost:8080',
      apiKey: 'test-key',
      timeout: 5000,
      maxRetries: 5,
    });
    expect(config.apiKey).toBe('test-key');
    expect(config.timeout).toBe(5000);
    expect(config.maxRetries).toBe(5);
  });

  it('should remove trailing slash from apiUrl', () => {
    const config = new MemoryConfig({ apiUrl: 'http://localhost:8080/' });
    expect(config.apiUrl).toBe('http://localhost:8080');
  });
});

describe('Models', () => {
  it('should have correct MemoryType values', () => {
    expect(MemoryType.PROFILE).toBe('profile');
    expect(MemoryType.EVENT).toBe('event');
    expect(MemoryType.KNOWLEDGE).toBe('knowledge');
    expect(MemoryType.BEHAVIOR).toBe('behavior');
    expect(MemoryType.SKILL).toBe('skill');
    expect(MemoryType.TOOL).toBe('tool');
    expect(MemoryType.CONVERSATION).toBe('conversation');
    expect(MemoryType.DOCUMENT).toBe('document');
  });

  it('should have correct Modality values', () => {
    expect(Modality.TEXT).toBe('text');
    expect(Modality.CONVERSATION).toBe('conversation');
    expect(Modality.DOCUMENT).toBe('document');
    expect(Modality.IMAGE).toBe('image');
    expect(Modality.VIDEO).toBe('video');
    expect(Modality.AUDIO).toBe('audio');
  });

  it('should have correct GraphQueryType values', () => {
    expect(GraphQueryType.CAUSAL_CHAIN).toBe('causal_chain');
    expect(GraphQueryType.TIMELINE).toBe('timeline');
    expect(GraphQueryType.TEMPORAL_BFS).toBe('temporal_bfs');
    expect(GraphQueryType.TEMPORAL_PATH).toBe('temporal_path');
  });
});

describe('Client instantiation', () => {
  it('should be able to instantiate client', async () => {
    // Dynamic import to verify module loads correctly
    const { EvifMemoryClient } = await import('../src/client');
    const config = new MemoryConfig({ apiUrl: 'http://localhost:8080' });
    const client = new EvifMemoryClient(config);
    expect(client).toBeDefined();
    await client.close();
  });

  it('should handle custom configuration', async () => {
    const { EvifMemoryClient } = await import('../src/client');
    const config = new MemoryConfig({
      apiUrl: 'http://localhost:8080',
      apiKey: 'secret-key',
      timeout: 10000,
      maxRetries: 5,
    });
    const client = new EvifMemoryClient(config);
    expect(client).toBeDefined();
    await client.close();
  });

  it('should post canonical graph query payloads and return the REST response shape', async () => {
    const graphResponse = {
      query_type: 'timeline',
      nodes: [
        {
          id: 'node-1',
          type: 'memory',
          label: 'First memory',
          timestamp: '2026-03-15T00:00:00Z',
        },
      ],
      timeline: [
        {
          node_id: 'node-1',
          timestamp: '2026-03-15T00:00:00Z',
          event_type: 'knowledge',
        },
      ],
      total: 1,
    };
    mockRequest.mockResolvedValueOnce({ data: graphResponse });

    const { EvifMemoryClient } = await import('../src/client');
    const client = new EvifMemoryClient(new MemoryConfig({ apiUrl: 'http://localhost:8080' }));
    const result = await client.queryGraph(GraphQueryType.TIMELINE, {
      startNode: 'node-1',
      maxDepth: 2,
      eventType: 'knowledge',
    });

    expect(axios.create).toHaveBeenCalled();
    expect(mockRequest).toHaveBeenCalledWith({
      method: 'POST',
      url: '/api/v1/graph/query',
      data: {
        query_type: 'timeline',
        start_node: 'node-1',
        end_node: undefined,
        max_depth: 2,
        event_type: 'knowledge',
        category: undefined,
        start_time: undefined,
        end_time: undefined,
      },
    });
    expect(result).toEqual(graphResponse);
  });

  it('should map legacy nodeId options onto start_node for graph queries', async () => {
    mockRequest.mockResolvedValueOnce({
      data: {
        query_type: 'causal_chain',
        nodes: [{ id: 'node-2', type: 'memory', label: 'Second memory' }],
        total: 1,
      },
    });

    const { EvifMemoryClient } = await import('../src/client');
    const client = new EvifMemoryClient(new MemoryConfig({ apiUrl: 'http://localhost:8080' }));
    await client.queryGraph(GraphQueryType.CAUSAL_CHAIN, {
      nodeId: 'node-2',
      maxDepth: 1,
      limit: 99,
    });

    expect(mockRequest).toHaveBeenCalledWith({
      method: 'POST',
      url: '/api/v1/graph/query',
      data: {
        query_type: 'causal_chain',
        start_node: 'node-2',
        end_node: undefined,
        max_depth: 1,
        event_type: undefined,
        category: undefined,
        start_time: undefined,
        end_time: undefined,
      },
    });
  });
});

describe('Type exports', () => {
  it('should export all required types', async () => {
    const module = await import('../src/index');
    expect(module.EvifMemoryClient).toBeDefined();
    expect(module.MemoryConfig).toBeDefined();
    expect(module.MemoryType).toBeDefined();
    expect(module.Modality).toBeDefined();
    expect(module.GraphQueryType).toBeDefined();
  });
});
