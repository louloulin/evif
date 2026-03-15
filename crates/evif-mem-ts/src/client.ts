/**
 * EVIF Memory Client - Main client for interacting with the EVIF Memory API.
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import { MemoryConfig } from './config';
import {
  Memory,
  MemorySearchResult,
  Category,
  GraphResult,
  SearchOptions,
  ListMemoriesOptions,
  GraphQueryOptions,
  MemoryType,
  Modality,
  GraphQueryType,
} from './models';

export class EvifMemoryClient {
  private readonly client: AxiosInstance;
  private readonly config: MemoryConfig;

  constructor(config: MemoryConfig) {
    this.config = config;
    this.client = axios.create({
      baseURL: config.apiUrl,
      timeout: config.timeout,
      headers: this.buildHeaders(),
    });
  }

  private buildHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'User-Agent': 'evif-mem-typescript/0.1.0',
    };
    if (this.config.apiKey) {
      headers['Authorization'] = `Bearer ${this.config.apiKey}`;
    }
    return headers;
  }

  private async request<T>(method: string, path: string, data?: unknown): Promise<T> {
    let retries = 0;
    let lastError: Error | null = null;

    while (retries <= this.config.maxRetries) {
      try {
        const response = await this.client.request<T>({
          method,
          url: path,
          data,
        });
        return response.data;
      } catch (error) {
        lastError = error as Error;
        retries++;
        if (retries <= this.config.maxRetries) {
          await new Promise((resolve) => setTimeout(resolve, 500 * retries));
        }
      }
    }

    throw new Error(`Request failed after ${this.config.maxRetries} retries: ${lastError?.message}`);
  }

  async createMemory(
    content: string,
    options?: {
      memoryType?: MemoryType | string;
      tags?: string[];
      modality?: Modality | string;
      metadata?: Record<string, unknown>;
    }
  ): Promise<Memory> {
    const data = {
      content,
      memory_type: options?.memoryType ?? MemoryType.KNOWLEDGE,
      tags: options?.tags ?? [],
      modality: options?.modality ?? Modality.TEXT,
      metadata: options?.metadata ?? {},
    };
    const result = await this.request<{ data: Memory }>('POST', '/api/v1/memories', data);
    return result.data;
  }

  async getMemory(memoryId: string): Promise<Memory> {
    const result = await this.request<{ data: Memory }>('GET', `/api/v1/memories/${memoryId}`);
    return result.data;
  }

  async listMemories(options?: ListMemoriesOptions): Promise<Memory[]> {
    const params = {
      limit: options?.limit ?? 100,
      offset: options?.offset ?? 0,
    };
    const result = await this.request<{ data: Memory[] }>('GET', '/api/v1/memories', { params });
    return result.data ?? [];
  }

  async searchMemories(query: string, searchOptions?: SearchOptions): Promise<MemorySearchResult[]> {
    const data = {
      query,
      k: searchOptions?.k ?? 10,
      threshold: searchOptions?.threshold ?? 0.0,
      mode: searchOptions?.mode ?? 'vector',
    };
    const result = await this.request<{ data: Array<{ memory: Memory; score: number }> }>(
      'POST',
      '/api/v1/memories/search',
      data
    );
    return (result.data ?? []).map((item) => ({
      memory: item.memory,
      score: item.score,
    }));
  }

  async deleteMemory(memoryId: string): Promise<boolean> {
    await this.request<void>('DELETE', `/api/v1/memories/${memoryId}`);
    return true;
  }

  async listCategories(): Promise<Category[]> {
    const result = await this.request<{ data: Category[] }>('GET', '/api/v1/categories');
    return result.data ?? [];
  }

  async getCategory(categoryId: string): Promise<Category> {
    const result = await this.request<{ data: Category }>('GET', `/api/v1/categories/${categoryId}`);
    return result.data;
  }

  async getCategoryMemories(categoryId: string, limit: number = 100): Promise<Memory[]> {
    const params = { limit };
    const result = await this.request<{ data: Memory[] }>(
      'GET',
      `/api/v1/categories/${categoryId}/memories`,
      { params }
    );
    return result.data ?? [];
  }

  async queryGraph(queryType: GraphQueryType | string, queryOptions?: GraphQueryOptions): Promise<GraphResult> {
    const data = {
      query_type: queryOptions?.queryType ?? queryType ?? GraphQueryType.CAUSAL_CHAIN,
      start_node: queryOptions?.startNode ?? queryOptions?.nodeId,
      end_node: queryOptions?.endNode,
      max_depth: queryOptions?.maxDepth ?? 5,
      event_type: queryOptions?.eventType,
      category: queryOptions?.category,
      start_time: queryOptions?.startTime,
      end_time: queryOptions?.endTime,
    };
    return this.request<GraphResult>('POST', '/api/v1/graph/query', data);
  }

  async close(): Promise<void> {
    // Axios doesn't require explicit closing for node.js
  }
}
