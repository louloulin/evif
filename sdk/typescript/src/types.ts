/**
 * EVIF TypeScript SDK Types
 * 
 * P3-5: TypeScript type definitions for EVIF API
 * Generated from OpenAPI spec
 */

// ============ Common Types ============

/** API Error response */
export interface ApiError {
  error: string;
  message: string;
  trace_id?: string;
}

/** Health check response */
export interface HealthResponse {
  status: 'ok' | 'error';
  version?: string;
  uptime?: number;
}

// ============ VFS Types ============

/** File information */
export interface FileInfo {
  name: string;
  size: number;
  mode: number;
  modified: string;
  is_dir: boolean;
}

/** Directory listing */
export interface DirectoryListing {
  path: string;
  entries: FileInfo[];
}

/** File read response */
export interface FileReadResponse {
  content: string;
  data: string; // Base64 encoded
  size: number;
}

/** File write request */
export interface FileWriteRequest {
  path: string;
  data: string;
  encoding?: 'plain' | 'base64';
}

/** File write response */
export interface FileWriteResponse {
  bytes_written: number;
  path: string;
}

/** Grep request */
export interface GrepRequest {
  path: string;
  pattern: string;
  max_results?: number;
  recursive?: boolean;
  trace?: boolean;
}

/** Grep match */
export interface GrepMatch {
  path: string;
  line_number: number;
  line: string;
}

/** Grep response */
export interface GrepResponse {
  pattern: string;
  matches: GrepMatch[];
  trace?: GrepTraceStep[];
}

/** Grep trace step */
export interface GrepTraceStep {
  path: string;
  operation: string;
  hits: number;
  latency_ms: number;
}

// ============ Memory Types ============

/** Memory item types */
export type MemoryType = 'profile' | 'knowledge' | 'context' | 'skill';

/** Memory item */
export interface MemoryItem {
  id: string;
  memory_type: MemoryType;
  content: string;
  created_at?: string;
  updated_at?: string;
  user_id?: string;
  tenant_id?: string;
}

/** Create memory item request */
export interface CreateMemoryItemRequest {
  memory_type: MemoryType;
  content: string;
  user_id?: string;
}

/** Memory search result */
export interface MemorySearchResult {
  item: MemoryItem;
  score: number;
}

/** Memory search request */
export interface MemorySearchRequest {
  query: string;
  limit?: number;
  memory_type?: MemoryType;
}

// ============ MCP Types ============

/** MCP tool definition */
export interface McpTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

/** MCP call request */
export interface McpCallRequest {
  tool: string;
  arguments: Record<string, unknown>;
}

/** MCP call result */
export interface McpCallResult {
  success: boolean;
  result?: unknown;
  error?: string;
}

// ============ Marketplace Types ============

/** Pricing type */
export type PricingType = 'free' | 'subscription' | 'purchase' | 'freenium';

/** Plugin pricing */
export interface PluginPricing {
  type: PricingType;
  monthly_price_cents?: number;
  price_cents?: number;
  free_tier?: boolean;
  pro_price_cents?: number;
}

/** Publisher information */
export interface PublisherInfo {
  user_id: string;
  display_name: string;
  verified: boolean;
}

/** Download statistics */
export interface DownloadStats {
  total: number;
  monthly: number;
  weekly: number;
}

/** Rating statistics */
export interface RatingStats {
  average: number;
  count: number;
  distribution?: Record<number, number>;
}

/** Marketplace status */
export type MarketplaceStatus = 'draft' | 'pending_review' | 'published' | 'suspended' | 'removed';

/** Marketplace entry */
export interface MarketplaceEntry {
  marketplace_id: string;
  plugin_id: string;
  publisher: PublisherInfo;
  pricing: PluginPricing;
  stats: DownloadStats;
  ratings: RatingStats;
  status: MarketplaceStatus;
  created_at: string;
  updated_at: string;
}

/** Marketplace search response */
export interface MarketplaceSearchResponse {
  plugins: MarketplaceEntry[];
  total: number;
  page: number;
  per_page: number;
  has_more: boolean;
}

/** Publish plugin request */
export interface PublishPluginRequest {
  plugin_id: string;
  description?: string;
  pricing: PluginPricing;
}

/** Sort options */
export type SortBy = 'popular' | 'recent' | 'rating' | 'price';

// ============ Billing Types ============

/** Pricing plan */
export type PricingPlan = 'free' | 'pro' | 'team' | 'enterprise';

/** Billing period */
export interface BillingPeriod {
  start: string;
  end: string;
}

/** Quota information */
export interface Quota {
  used: number;
  limit: number;
  unit: string;
}

/** Usage statistics */
export interface UsageStats {
  api_calls: Quota;
  storage: Quota;
  agents: Quota;
}

/** Usage response */
export interface UsageResponse {
  tenant_id: string;
  period: BillingPeriod;
  plan: PricingPlan;
  quotas: UsageStats;
}

/** Usage history entry */
export interface UsageHistoryEntry {
  date: string;
  api_calls: number;
  storage_bytes: number;
  active_agents: number;
}

/** Usage detail */
export interface UsageDetail {
  endpoint: string;
  count: number;
  total_latency_ms: number;
}

/** Subscription status */
export type SubscriptionStatus = 'active' | 'trial' | 'cancelled' | 'past_due';

/** Subscription */
export interface Subscription {
  tenant_id: string;
  plan: PricingPlan;
  started_at: string;
  next_billing_at: string;
  status: SubscriptionStatus;
}

/** Invoice status */
export type InvoiceStatus = 'draft' | 'paid' | 'unpaid' | 'void';

/** Invoice */
export interface Invoice {
  id: string;
  tenant_id: string;
  period: BillingPeriod;
  amount_cents: number;
  status: InvoiceStatus;
  created_at: string;
  pdf_url?: string;
}

/** Webhook event types */
export type WebhookEvent = 'quota_80_percent' | 'quota_100_percent' | 'subscription_changed' | 'invoice_created';

/** Webhook configuration */
export interface WebhookConfig {
  id: string;
  tenant_id: string;
  url: string;
  events: WebhookEvent[];
  secret: string;
}

// ============ Admin Types ============

/** Global statistics */
export interface GlobalStats {
  total_tenants: number;
  active_tenants: number;
  total_api_calls_today: number;
  total_storage_bytes: number;
  revenue_mtd_cents: number;
  active_plugins: number;
  marketplace_gmv_cents: number;
}

/** Tenant statistics */
export interface TenantStats {
  tenant_id: string;
  display_name: string;
  plan: string;
  api_calls_today: number;
  storage_bytes: number;
  user_count: number;
  active: boolean;
}

/** Plugin statistics */
export interface PluginStats {
  plugin_id: string;
  name: string;
  install_count: number;
  active_installs: number;
  revenue_cents: number;
}

/** Revenue statistics */
export interface RevenueStats {
  total_revenue_cents: number;
  mrr_cents: number;
  arr_cents: number;
  new_customers: number;
  churned_customers: number;
}

/** Actor type */
export type ActorType = 'user' | 'system' | 'admin';

/** Audit entry */
export interface AuditEntry {
  id: string;
  timestamp: string;
  actor_id: string;
  actor_type: ActorType;
  action: string;
  resource_type: string;
  resource_id: string;
  details: Record<string, unknown>;
  ip_address?: string;
}

/** User role */
export type UserRole = 'member' | 'admin' | 'superadmin';

/** Admin user */
export interface AdminUser {
  id: string;
  tenant_id: string;
  email: string;
  name: string;
  role: UserRole;
  active: boolean;
  created_at: string;
  last_login?: string;
}

/** Create user request */
export interface CreateUserRequest {
  email: string;
  name: string;
  role: UserRole;
}

/** Update user request */
export interface UpdateUserRequest {
  name?: string;
  role?: UserRole;
  active?: boolean;
}

// ============ Client Configuration ============

/** EVIF client configuration */
export interface EvifClientConfig {
  /** API base URL */
  baseUrl: string;
  /** API key for authentication */
  apiKey?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Retry configuration */
  retry?: {
    maxRetries: number;
    backoffMs: number;
  };
}

/** Request options */
export interface RequestOptions {
  headers?: Record<string, string>;
  timeout?: number;
}
