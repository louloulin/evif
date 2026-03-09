"use strict";
/**
 * Configuration options for the EVIF Memory client.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MemoryConfig = void 0;
class MemoryConfig {
    constructor(options) {
        this.apiUrl = options.apiUrl.replace(/\/$/, ''); // Remove trailing slash
        this.apiKey = options.apiKey;
        this.timeout = options.timeout ?? 30000;
        this.maxRetries = options.maxRetries ?? 3;
    }
}
exports.MemoryConfig = MemoryConfig;
