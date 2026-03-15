/**
 * Data models for EVIF Memory API.
 */
export declare enum MemoryType {
    PROFILE = "profile",
    EVENT = "event",
    KNOWLEDGE = "knowledge",
    BEHAVIOR = "behavior",
    SKILL = "skill",
    TOOL = "tool",
    CONVERSATION = "conversation",
    DOCUMENT = "document"
}
export declare enum Modality {
    TEXT = "text",
    CONVERSATION = "conversation",
    DOCUMENT = "document",
    IMAGE = "image",
    VIDEO = "video",
    AUDIO = "audio"
}
export declare enum GraphQueryType {
    CAUSAL_CHAIN = "causal_chain",
    TIMELINE = "timeline",
    TEMPORAL_BFS = "temporal_bfs",
    TEMPORAL_PATH = "temporal_path"
}
export interface MemoryCreate {
    content: string;
    memoryType?: MemoryType | string;
    tags?: string[];
    modality?: Modality | string;
    references?: string[];
    metadata?: Record<string, unknown>;
}
export interface Memory {
    id: string;
    content: string;
    summary: string;
    memory_type: MemoryType | string;
    tags: string[];
    embedding?: number[];
    reinforcement_count: number;
    last_reinforced_at?: string;
    created_at: string;
    updated_at: string;
    references: string[];
    user_id?: string;
    tenant_id?: string;
    metadata: Record<string, unknown>;
}
export interface MemorySearchResult {
    memory: Memory;
    score: number;
}
export interface Category {
    id: string;
    name: string;
    description: string;
    summary: string;
    item_count: number;
    embedding?: number[];
    created_at: string;
    updated_at: string;
}
export interface GraphQuery {
    query_type: GraphQueryType | string;
    start_node?: string;
    end_node?: string;
    max_depth?: number;
    event_type?: string;
    category?: string;
    start_time?: string;
    end_time?: string;
    /** @deprecated Use start_node. */
    node_id?: string;
    /** @deprecated The REST graph API ignores this field. */
    limit?: number;
}
export interface GraphNode {
    id: string;
    type: string;
    label: string;
    timestamp?: string;
}
export interface TimelineEvent {
    node_id: string;
    timestamp: string;
    event_type: string;
}
export interface GraphPathInfo {
    nodes: string[];
    edges: string[];
    narrative: string;
}
export interface GraphResult {
    query_type: string;
    nodes?: GraphNode[];
    paths?: GraphPathInfo[];
    timeline?: TimelineEvent[];
    total: number;
}
export interface SearchOptions {
    k?: number;
    threshold?: number;
    mode?: 'vector' | 'hybrid' | 'rag';
}
export interface ListMemoriesOptions {
    limit?: number;
    offset?: number;
}
export interface GraphQueryOptions {
    queryType?: GraphQueryType | string;
    startNode?: string;
    endNode?: string;
    maxDepth?: number;
    eventType?: string;
    category?: string;
    startTime?: string;
    endTime?: string;
    /** @deprecated Use startNode. */
    nodeId?: string;
    /** @deprecated The REST graph API ignores this field. */
    limit?: number;
}
