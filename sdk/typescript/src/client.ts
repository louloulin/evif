/**
 * EVIF TypeScript SDK Client
 * 
 * P3-5: TypeScript SDK for EVIF API
 */

import type {
  EvifClientConfig,
  HealthResponse,
  FileWriteRequest,
  FileWriteResponse,
  GrepRequest,
  GrepResponse,
  MemoryItem,
  CreateMemoryItemRequest,
  MemorySearchRequest,
  MemorySearchResult,
  McpTool,
  McpCallRequest,
  McpCallResult,
  MarketplaceEntry,
  MarketplaceSearchResponse,
  PublishPluginRequest,
  UsageResponse,
  UsageHistoryEntry,
  Subscription,
  Invoice,
  WebhookConfig,
  GlobalStats,
  TenantStats,
  AuditEntry,
  AdminUser,
  CreateUserRequest,
  UpdateUserRequest,
  ApiError,
} from './types';

/**
 * EVIF API Client
 */
export class EvifClient {
  private config: Required<EvifClientConfig>;
  private apiKey: string;

  constructor(config: EvifClientConfig) {
    this.config = {
      baseUrl: config.baseUrl.replace(/\/$/, ''),
      timeout: config.timeout ?? 30000,
      retry: config.retry ?? { maxRetries: 3, backoffMs: 1000 },
    };
    this.apiKey = config.apiKey ?? '';
  }

  /**
   * Set the API key for authentication
   */
  setApiKey(apiKey: string): void {
    this.apiKey = apiKey;
  }

  // ============ Health Endpoints ============

  /** Get health status */
  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>('/health');
  }

  /** Get detailed health status */
  async healthV1(): Promise<HealthResponse> {
    return this.get<HealthResponse>('/api/v1/health');
  }

  // ============ VFS Endpoints ============

  /** Read file */
  async readFile(path: string, offset?: number, size?: number): Promise<string> {
    const params = new URLSearchParams({ path });
    if (offset !== undefined) params.set('offset', offset.toString());
    if (size !== undefined) params.set('size', size.toString());
    const result = await this.get<{ content: string }>(`/api/v1/fs/read?${params}`);
    return result.content;
  }

  /** Write file */
  async writeFile(request: FileWriteRequest): Promise<FileWriteResponse> {
    return this.post<FileWriteResponse>('/api/v1/fs/write', request);
  }

  /** List directory */
  async listDirectory(path: string): Promise<{ entries: Array<{ name: string; size: number; mode: number; modified: string; is_dir: boolean }> }> {
    const params = new URLSearchParams({ path });
    return this.get(`/api/v1/fs/list?${params}`);
  }

  /** Create directory */
  async createDirectory(path: string, perm?: number): Promise<void> {
    await this.post('/api/v1/fs/mkdir', { path, perm });
  }

  /** Delete file or directory */
  async deleteFile(path: string, recursive?: boolean): Promise<void> {
    const params = new URLSearchParams({ path });
    if (recursive !== undefined) params.set('recursive', recursive.toString());
    await this.delete(`/api/v1/fs/delete?${params}`);
  }

  /** Search files with regex */
  async grep(request: GrepRequest): Promise<GrepResponse> {
    return this.post<GrepResponse>('/api/v1/grep', request);
  }

  // ============ Memory Endpoints ============

  /** List memory items */
  async listMemoryItems(userId?: string, memoryType?: string): Promise<MemoryItem[]> {
    const params = new URLSearchParams();
    if (userId) params.set('user_id', userId);
    if (memoryType) params.set('memory_type', memoryType);
    const query = params.toString();
    return this.get<MemoryItem[]>(`/api/v1/memory/items${query ? '?' + query : ''}`);
  }

  /** Create memory item */
  async createMemoryItem(request: CreateMemoryItemRequest): Promise<MemoryItem> {
    return this.post<MemoryItem>('/api/v1/memory/items', request);
  }

  /** Search memory */
  async searchMemory(request: MemorySearchRequest): Promise<MemorySearchResult[]> {
    return this.post<MemorySearchResult[]>('/api/v1/memory/search', request);
  }

  // ============ MCP Endpoints ============

  /** List MCP tools */
  async listMcpTools(): Promise<McpTool[]> {
    return this.get<McpTool[]>('/api/v1/mcp/tools');
  }

  /** Call MCP tool */
  async callMcpTool(request: McpCallRequest): Promise<McpCallResult> {
    return this.post<McpCallResult>('/api/v1/mcp/call', request);
  }

  // ============ Marketplace Endpoints ============

  /** Search plugins */
  async searchPlugins(params?: {
    q?: string;
    tier?: string;
    page?: number;
    per_page?: number;
  }): Promise<MarketplaceSearchResponse> {
    const query = new URLSearchParams(params as Record<string, string>).toString();
    return this.get<MarketplaceSearchResponse>(`/api/v1/marketplace/plugins${query ? '?' + query : ''}`);
  }

  /** Get plugin by ID */
  async getPlugin(id: string): Promise<MarketplaceEntry> {
    return this.get<MarketplaceEntry>(`/api/v1/marketplace/plugins/${id}`);
  }

  /** Publish plugin */
  async publishPlugin(request: PublishPluginRequest): Promise<MarketplaceEntry> {
    return this.post<MarketplaceEntry>('/api/v1/marketplace/plugins', request);
  }

  /** Get trending plugins */
  async getTrendingPlugins(): Promise<MarketplaceEntry[]> {
    return this.get<MarketplaceEntry[]>('/api/v1/marketplace/trending');
  }

  /** Get free plugins */
  async getFreePlugins(): Promise<MarketplaceEntry[]> {
    return this.get<MarketplaceEntry[]>('/api/v1/marketplace/free');
  }

  // ============ Billing Endpoints ============

  /** Get usage for tenant */
  async getUsage(tenantId: string): Promise<UsageResponse> {
    return this.get<UsageResponse>(`/api/v1/billing/usage/${tenantId}`);
  }

  /** Get usage history */
  async getUsageHistory(tenantId: string, months?: number): Promise<UsageHistoryEntry[]> {
    const params = new URLSearchParams();
    if (months !== undefined) params.set('months', months.toString());
    const query = params.toString();
    return this.get<UsageHistoryEntry[]>(`/api/v1/billing/usage/history/${tenantId}${query ? '?' + query : ''}`);
  }

  /** Get subscription */
  async getSubscription(tenantId: string): Promise<Subscription> {
    return this.get<Subscription>(`/api/v1/billing/subscription/${tenantId}`);
  }

  /** Create subscription */
  async createSubscription(tenantId: string, plan: string): Promise<Subscription> {
    return this.post<Subscription>(`/api/v1/billing/subscription/${tenantId}`, { plan });
  }

  /** Get invoices */
  async getInvoices(tenantId: string): Promise<Invoice[]> {
    return this.get<Invoice[]>(`/api/v1/billing/invoices/${tenantId}`);
  }

  // ============ Admin Endpoints ============

  /** Get global overview stats */
  async getOverviewStats(): Promise<GlobalStats> {
    return this.get<GlobalStats>('/api/v1/admin/stats/overview');
  }

  /** Get tenant stats */
  async getTenantStats(): Promise<TenantStats[]> {
    return this.get<TenantStats[]>('/api/v1/admin/stats/tenants');
  }

  /** List users for tenant */
  async listUsers(tenantId: string): Promise<AdminUser[]> {
    return this.get<AdminUser[]>(`/api/v1/admin/users/${tenantId}`);
  }

  /** Create user */
  async createUser(tenantId: string, request: CreateUserRequest): Promise<AdminUser> {
    return this.post<AdminUser>(`/api/v1/admin/users/${tenantId}`, request);
  }

  /** Update user */
  async updateUser(tenantId: string, userId: string, request: UpdateUserRequest): Promise<AdminUser> {
    return this.put<AdminUser>(`/api/v1/admin/users/${tenantId}/${userId}`, request);
  }

  /** Delete user */
  async deleteUser(tenantId: string, userId: string): Promise<void> {
    await this.delete(`/api/v1/admin/users/${tenantId}/${userId}`);
  }

  /** Get audit log */
  async getAuditLog(params?: { limit?: number; offset?: number }): Promise<AuditEntry[]> {
    const query = new URLSearchParams(params as Record<string, string>).toString();
    return this.get<AuditEntry[]>(`/api/v1/admin/audit${query ? '?' + query : ''}`);
  }

  // ============ Private Methods ============

  private async get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  private async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  private async put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PUT', path, body);
  }

  private async delete(path: string): Promise<void> {
    await this.request('DELETE', path);
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.config.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    const response = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: response.statusText }));
      throw new Error((error as ApiError).message || `HTTP ${response.status}`);
    }

    return response.json();
  }
}

/** Default export */
export default EvifClient;
export { EvifClient as Client };
export * from './types';
