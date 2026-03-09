"use strict";
/**
 * EVIF Memory Client - Main client for interacting with the EVIF Memory API.
 */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.EvifMemoryClient = void 0;
const axios_1 = __importDefault(require("axios"));
const models_1 = require("./models");
class EvifMemoryClient {
    constructor(config) {
        this.config = config;
        this.client = axios_1.default.create({
            baseURL: config.apiUrl,
            timeout: config.timeout,
            headers: this.buildHeaders(),
        });
    }
    buildHeaders() {
        const headers = {
            'Content-Type': 'application/json',
            'User-Agent': 'evif-mem-typescript/0.1.0',
        };
        if (this.config.apiKey) {
            headers['Authorization'] = `Bearer ${this.config.apiKey}`;
        }
        return headers;
    }
    async request(method, path, data) {
        let retries = 0;
        let lastError = null;
        while (retries <= this.config.maxRetries) {
            try {
                const response = await this.client.request({
                    method,
                    url: path,
                    data,
                });
                return response.data;
            }
            catch (error) {
                lastError = error;
                retries++;
                if (retries <= this.config.maxRetries) {
                    await new Promise((resolve) => setTimeout(resolve, 500 * retries));
                }
            }
        }
        throw new Error(`Request failed after ${this.config.maxRetries} retries: ${lastError?.message}`);
    }
    async createMemory(content, options) {
        const data = {
            content,
            memory_type: options?.memoryType ?? models_1.MemoryType.KNOWLEDGE,
            tags: options?.tags ?? [],
            modality: options?.modality ?? models_1.Modality.TEXT,
            metadata: options?.metadata ?? {},
        };
        const result = await this.request('POST', '/api/v1/memories', data);
        return result.data;
    }
    async getMemory(memoryId) {
        const result = await this.request('GET', `/api/v1/memories/${memoryId}`);
        return result.data;
    }
    async listMemories(options) {
        const params = {
            limit: options?.limit ?? 100,
            offset: options?.offset ?? 0,
        };
        const result = await this.request('GET', '/api/v1/memories', { params });
        return result.data ?? [];
    }
    async searchMemories(query, searchOptions) {
        const data = {
            query,
            k: searchOptions?.k ?? 10,
            threshold: searchOptions?.threshold ?? 0.0,
            mode: searchOptions?.mode ?? 'vector',
        };
        const result = await this.request('POST', '/api/v1/memories/search', data);
        return (result.data ?? []).map((item) => ({
            memory: item.memory,
            score: item.score,
        }));
    }
    async deleteMemory(memoryId) {
        await this.request('DELETE', `/api/v1/memories/${memoryId}`);
        return true;
    }
    async listCategories() {
        const result = await this.request('GET', '/api/v1/categories');
        return result.data ?? [];
    }
    async getCategory(categoryId) {
        const result = await this.request('GET', `/api/v1/categories/${categoryId}`);
        return result.data;
    }
    async getCategoryMemories(categoryId, limit = 100) {
        const params = { limit };
        const result = await this.request('GET', `/api/v1/categories/${categoryId}/memories`, { params });
        return result.data ?? [];
    }
    async queryGraph(query, queryOptions) {
        const data = {
            query,
            query_type: queryOptions?.queryType ?? models_1.GraphQueryType.CAUSAL_CHAIN,
            node_id: queryOptions?.nodeId,
            max_depth: queryOptions?.maxDepth ?? 3,
            limit: queryOptions?.limit ?? 10,
        };
        const result = await this.request('POST', '/api/v1/graph/query', data);
        return result.data;
    }
    async close() {
        // Axios doesn't require explicit closing for node.js
    }
}
exports.EvifMemoryClient = EvifMemoryClient;
