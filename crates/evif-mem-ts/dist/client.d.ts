/**
 * EVIF Memory Client - Main client for interacting with the EVIF Memory API.
 */
import { MemoryConfig } from './config';
import { Memory, MemorySearchResult, Category, GraphResult, SearchOptions, ListMemoriesOptions, GraphQueryOptions, MemoryType, Modality, GraphQueryType } from './models';
export declare class EvifMemoryClient {
    private readonly client;
    private readonly config;
    constructor(config: MemoryConfig);
    private buildHeaders;
    private request;
    createMemory(content: string, options?: {
        memoryType?: MemoryType | string;
        tags?: string[];
        modality?: Modality | string;
        metadata?: Record<string, unknown>;
    }): Promise<Memory>;
    getMemory(memoryId: string): Promise<Memory>;
    listMemories(options?: ListMemoriesOptions): Promise<Memory[]>;
    searchMemories(query: string, searchOptions?: SearchOptions): Promise<MemorySearchResult[]>;
    deleteMemory(memoryId: string): Promise<boolean>;
    listCategories(): Promise<Category[]>;
    getCategory(categoryId: string): Promise<Category>;
    getCategoryMemories(categoryId: string, limit?: number): Promise<Memory[]>;
    queryGraph(queryType: GraphQueryType | string, queryOptions?: GraphQueryOptions): Promise<GraphResult>;
    close(): Promise<void>;
}
