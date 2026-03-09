/**
 * Unit tests for EvifMemoryClient
 */

import { describe, it, expect } from 'vitest';
import { MemoryConfig } from '../src/config';
import { MemoryType, Modality, GraphQueryType } from '../src/models';

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
