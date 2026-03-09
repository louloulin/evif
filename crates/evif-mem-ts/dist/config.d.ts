/**
 * Configuration options for the EVIF Memory client.
 */
export interface MemoryConfigOptions {
    /** Base URL of the EVIF Memory API */
    apiUrl: string;
    /** Optional API key for authentication */
    apiKey?: string;
    /** Request timeout in milliseconds (default: 30000) */
    timeout?: number;
    /** Maximum number of retry attempts (default: 3) */
    maxRetries?: number;
}
export declare class MemoryConfig {
    readonly apiUrl: string;
    readonly apiKey?: string;
    readonly timeout: number;
    readonly maxRetries: number;
    constructor(options: MemoryConfigOptions);
}
