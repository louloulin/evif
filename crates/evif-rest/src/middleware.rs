// REST API 中间件

use axum::{
    extract::{Request, State},
    http::{
        header::{AUTHORIZATION, USER_AGENT},
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use evif_auth::{
    AuditEvent, AuditEventType, AuditFilter, AuditLogManager, AuthManager, AuthPolicy, Capability,
    JwtValidator, Permission, Permissions, Principal,
};
use parking_lot::Mutex;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::warn;
use uuid::Uuid;

use crate::tenant_handlers::{TenantState, TENANT_HEADER};
use crate::TrafficStats;

const REQUEST_ID_HEADER: &str = "x-request-id";
const CORRELATION_ID_HEADER: &str = "x-correlation-id";
const RATE_LIMIT_LIMIT_HEADER: &str = "x-ratelimit-limit";
const RATE_LIMIT_REMAINING_HEADER: &str = "x-ratelimit-remaining";
const RETRY_AFTER_HEADER: &str = "retry-after";
const RETRY_AFTER_SECS: u64 = 1;

fn write_scope_id() -> Uuid {
    Uuid::from_u128(0x8f53_7321_9f58_4a6f_8d21_3141_0000_0001)
}

fn admin_scope_id() -> Uuid {
    Uuid::from_u128(0x8f53_7321_9f58_4a6f_8d21_3141_0000_0002)
}

#[derive(Clone)]
struct ApiKeyIdentity {
    principal: Principal,
    principal_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestIdentity {
    pub request_id: String,
    pub correlation_id: String,
}

struct RestAuthInner {
    manager: AuthManager,
    api_keys: HashMap<String, ApiKeyIdentity>,
    hashed_api_keys: HashMap<String, ApiKeyIdentity>,
    audit_log: Arc<AuditLogManager>,
    enforce: bool,
    api_key_max_concurrent_requests: Option<usize>,
    api_key_limiters: Mutex<HashMap<Uuid, Arc<Semaphore>>>,
    jwt_validator: Option<JwtValidator>,
}

/// REST 认证共享状态
#[derive(Clone)]
pub struct RestAuthState {
    inner: Arc<RestAuthInner>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RouteRequirement {
    permission: Permission,
    scope: &'static str,
    resource_id: Uuid,
}

enum AuthDecision {
    Granted(ApiKeyIdentity),
    MissingCredentials,
    InvalidCredentials,
    Forbidden(ApiKeyIdentity),
    Internal(String),
}

struct ApiKeyRateLimitLease {
    _permit: OwnedSemaphorePermit,
    limit: usize,
    remaining: usize,
}

#[derive(Clone)]
pub(crate) struct IpRateLimitState {
    max_concurrent_requests: Option<usize>,
    limiters: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
}

impl IpRateLimitState {
    pub(crate) fn from_env() -> Self {
        Self::new(parse_optional_usize_env(
            "EVIF_REST_IP_MAX_CONCURRENT_REQUESTS",
        ))
    }

    fn new(max_concurrent_requests: Option<usize>) -> Self {
        Self {
            max_concurrent_requests,
            limiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn try_acquire(&self, ip: &str) -> Result<Option<ApiKeyRateLimitLease>, usize> {
        let Some(limit) = self.max_concurrent_requests else {
            return Ok(None);
        };

        let semaphore = {
            let mut limiters = self.limiters.lock();
            limiters
                .entry(ip.to_string())
                .or_insert_with(|| Arc::new(Semaphore::new(limit)))
                .clone()
        };

        match semaphore.clone().try_acquire_owned() {
            Ok(permit) => Ok(Some(ApiKeyRateLimitLease {
                _permit: permit,
                limit,
                remaining: semaphore.available_permits(),
            })),
            Err(_) => Err(limit),
        }
    }
}

impl RestAuthState {
    /// 从环境变量构造 REST 认证配置。
    ///
    /// - `EVIF_REST_AUTH_MODE=disabled|off|false` 可关闭鉴权（默认开启）
    /// - `EVIF_REST_WRITE_API_KEYS=key1,key2`
    /// - `EVIF_REST_ADMIN_API_KEYS=key3,key4`
    /// - `EVIF_REST_AUTH_AUDIT_LOG=/path/to/evif-audit.log`
    /// - `EVIF_REST_JWT_SECRET=<secret>` - JWT 签名密钥
    /// - `EVIF_REST_JWT_ISSUER=<issuer>` - JWT issuer（可选）
    /// - `EVIF_REST_JWT_AUDIENCE=<audience>` - JWT audience（可选）
    pub fn from_env() -> Self {
        let enforce = std::env::var("EVIF_REST_AUTH_MODE")
            .map(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "disabled" | "off" | "false"
                )
            })
            .unwrap_or(true);

        let write_keys = parse_key_list("EVIF_REST_WRITE_API_KEYS");
        let admin_keys = parse_key_list("EVIF_REST_ADMIN_API_KEYS");
        let write_key_hashes = parse_hashed_key_list("EVIF_REST_WRITE_API_KEYS_SHA256");
        let admin_key_hashes = parse_hashed_key_list("EVIF_REST_ADMIN_API_KEYS_SHA256");
        let audit_log = audit_log_from_env();
        let api_key_max_concurrent_requests =
            parse_optional_usize_env("EVIF_REST_API_KEY_MAX_CONCURRENT_REQUESTS");

        // 尝试加载 JWT 配置
        let jwt_validator = jwt_validator_from_env();

        Self::new(
            write_keys,
            admin_keys,
            write_key_hashes,
            admin_key_hashes,
            audit_log,
            enforce,
            api_key_max_concurrent_requests,
            jwt_validator,
        )
    }

    pub fn disabled() -> Self {
        Self::new(
            vec![],
            vec![],
            vec![],
            vec![],
            Arc::new(AuditLogManager::from_memory()),
            false,
            None,
            None,
        )
    }

    pub fn from_api_keys(
        write_keys: impl IntoIterator<Item = String>,
        admin_keys: impl IntoIterator<Item = String>,
    ) -> Self {
        Self::from_api_keys_with_concurrency_limit(write_keys, admin_keys, 0)
    }

    pub fn from_api_keys_with_concurrency_limit(
        write_keys: impl IntoIterator<Item = String>,
        admin_keys: impl IntoIterator<Item = String>,
        max_concurrent_requests: usize,
    ) -> Self {
        Self::new(
            write_keys.into_iter().collect(),
            admin_keys.into_iter().collect(),
            vec![],
            vec![],
            Arc::new(AuditLogManager::from_memory()),
            true,
            normalize_rate_limit(max_concurrent_requests),
            None,
        )
    }

    /// 从 API Keys 和 JWT 验证器创建
    pub fn from_api_keys_with_jwt(
        write_keys: impl IntoIterator<Item = String>,
        admin_keys: impl IntoIterator<Item = String>,
        jwt_validator: JwtValidator,
    ) -> Self {
        Self::new(
            write_keys.into_iter().collect(),
            admin_keys.into_iter().collect(),
            vec![],
            vec![],
            Arc::new(AuditLogManager::from_memory()),
            true,
            None,
            Some(jwt_validator),
        )
    }

    pub fn audit_events(&self) -> Vec<AuditEvent> {
        self.inner
            .audit_log
            .query(AuditFilter::new())
            .unwrap_or_default()
    }

    fn new(
        write_keys: Vec<String>,
        admin_keys: Vec<String>,
        write_key_hashes: Vec<String>,
        admin_key_hashes: Vec<String>,
        audit_log: Arc<AuditLogManager>,
        enforce: bool,
        api_key_max_concurrent_requests: Option<usize>,
        jwt_validator: Option<JwtValidator>,
    ) -> Self {
        let mut manager = AuthManager::with_policy(AuthPolicy::Strict);
        let mut api_keys = HashMap::new();
        let mut hashed_api_keys = HashMap::new();

        if enforce {
            for key in write_keys {
                register_key(&manager, &mut api_keys, key, false);
            }

            for key in admin_keys {
                register_key(&manager, &mut api_keys, key, true);
            }

            for key_hash in write_key_hashes {
                register_hashed_key(&manager, &mut hashed_api_keys, key_hash, false);
            }

            for key_hash in admin_key_hashes {
                register_hashed_key(&manager, &mut hashed_api_keys, key_hash, true);
            }
        }

        // 如果提供了 JWT 验证器，配置到 manager
        if let Some(validator) = jwt_validator.clone() {
            manager.set_jwt_validator(validator);
        }

        Self {
            inner: Arc::new(RestAuthInner {
                manager,
                api_keys,
                hashed_api_keys,
                audit_log,
                enforce,
                api_key_max_concurrent_requests,
                api_key_limiters: Mutex::new(HashMap::new()),
                jwt_validator,
            }),
        }
    }

    fn is_enforced(&self) -> bool {
        self.inner.enforce
    }

    fn try_acquire_rate_limit(
        &self,
        principal_id: Uuid,
    ) -> Result<Option<ApiKeyRateLimitLease>, usize> {
        let Some(limit) = self.inner.api_key_max_concurrent_requests else {
            return Ok(None);
        };

        let semaphore = {
            let mut limiters = self.inner.api_key_limiters.lock();
            limiters
                .entry(principal_id)
                .or_insert_with(|| Arc::new(Semaphore::new(limit)))
                .clone()
        };

        match semaphore.clone().try_acquire_owned() {
            Ok(permit) => Ok(Some(ApiKeyRateLimitLease {
                _permit: permit,
                limit,
                remaining: semaphore.available_permits(),
            })),
            Err(_) => Err(limit),
        }
    }

    fn authorize(&self, headers: &HeaderMap, requirement: RouteRequirement) -> AuthDecision {
        // 尝试 JWT 认证（如果配置了）
        if let Some(auth_value) = header_value(headers, AUTHORIZATION) {
            if let Some(jwt_validator) = self.inner.manager.jwt_validator() {
                match jwt_validator.validate(auth_value) {
                    Ok(claims) => {
                        // JWT 验证成功，创建用户身份
                        let user_id = Uuid::parse_str(&claims.sub)
                            .unwrap_or_else(|_| Uuid::new_v4());
                        let identity = ApiKeyIdentity {
                            principal: Principal::User(user_id),
                            principal_id: user_id,
                        };
                        return AuthDecision::Granted(identity);
                    }
                    Err(_) => {
                        // JWT 验证失败，继续尝试 API Key
                    }
                }
            }
        }

        // 尝试 API Key 认证
        let Some(api_key) = extract_api_key(headers) else {
            return AuthDecision::MissingCredentials;
        };

        let identity = self.inner.api_keys.get(api_key).cloned().or_else(|| {
            let api_key_hash = sha256_hex(api_key);
            self.inner.hashed_api_keys.get(&api_key_hash).cloned()
        });

        let Some(identity) = identity else {
            return AuthDecision::InvalidCredentials;
        };

        match self.inner.manager.check(
            &identity.principal,
            &requirement.resource_id,
            requirement.permission,
        ) {
            Ok(true) => AuthDecision::Granted(identity),
            Ok(false) => AuthDecision::Forbidden(identity),
            Err(err) => AuthDecision::Internal(err.to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_event(
        &self,
        event_type: AuditEventType,
        principal_id: Option<Uuid>,
        requirement: RouteRequirement,
        method: &Method,
        path: &str,
        success: bool,
        headers: &HeaderMap,
        reason: &str,
    ) {
        let mut event = AuditEvent::new(
            event_type,
            format!(
                "action={} path={} scope={} permission={:?} result={} reason={}",
                method,
                path,
                requirement.scope,
                requirement.permission,
                if success { "allowed" } else { "denied" },
                reason
            ),
        )
        .with_resource_id(requirement.resource_id)
        .with_success(success);

        if let Some(principal_id) = principal_id {
            event = event.with_principal_id(principal_id);
        }

        if let Some(user_agent) = header_value(headers, USER_AGENT) {
            event = event.with_user_agent(user_agent.to_string());
        }

        if let Some(ip_address) =
            header_value(headers, "x-forwarded-for").or_else(|| header_value(headers, "x-real-ip"))
        {
            event = event.with_ip_address(ip_address.to_string());
        }

        let _ = self.inner.audit_log.logger().log(event);
    }
}

/// N10: Structured logging middleware — emits structured fields to tracing.
#[allow(non_snake_case)]
pub async fn LoggingMiddleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    tracing::info!(method = %method, path = %uri, "HTTP request started");
    next.run(req).await
}

/// 为每个请求分配 request id / correlation id，并在响应头中返回。
#[allow(non_snake_case)]
pub async fn RequestIdentityMiddleware(mut req: Request, next: Next) -> Response {
    let identity = resolve_request_identity(req.headers());

    if let Ok(value) = HeaderValue::from_str(&identity.request_id) {
        req.headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), value);
    }
    if let Ok(value) = HeaderValue::from_str(&identity.correlation_id) {
        req.headers_mut()
            .insert(HeaderName::from_static(CORRELATION_ID_HEADER), value);
    }

    req.extensions_mut().insert(identity.clone());

    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&identity.request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), value);
    }
    if let Ok(value) = HeaderValue::from_str(&identity.correlation_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(CORRELATION_ID_HEADER), value);
    }

    response
}

#[allow(non_snake_case)]
pub async fn IpRateLimitMiddleware(
    State(ip_rate_limit_state): State<Arc<IpRateLimitState>>,
    req: Request,
    next: Next,
) -> Response {
    let ip = header_value(req.headers(), "x-forwarded-for")
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| header_value(req.headers(), "x-real-ip"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let lease = match ip.as_deref() {
        Some(ip) => match ip_rate_limit_state.try_acquire(ip) {
            Ok(lease) => lease,
            Err(limit) => return rate_limit_response(limit, "IP concurrency limit exceeded"),
        },
        None => None,
    };

    let mut response = next.run(req).await;
    if let Some(lease) = lease {
        apply_rate_limit_headers_if_missing(&mut response, lease.limit, lease.remaining);
    }
    response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrafficOperation {
    Read,
    Write,
    List,
    Other,
}

fn classify_traffic_operation(method: &Method, path: &str) -> Option<TrafficOperation> {
    if path == "/metrics" || path.starts_with("/api/v1/metrics/") {
        return None;
    }

    if *method == Method::GET && path == "/api/v1/files" {
        return Some(TrafficOperation::Read);
    }

    if *method == Method::GET && path == "/api/v1/directories" {
        return Some(TrafficOperation::List);
    }

    if matches!(*method, Method::POST | Method::PUT | Method::DELETE)
        && matches!(path, "/api/v1/files" | "/api/v1/directories")
    {
        return Some(TrafficOperation::Write);
    }

    Some(TrafficOperation::Other)
}

/// 流量统计中间件
/// 将真实 HTTP 请求接到 TrafficStats，避免 metrics 端点成为空壳。
#[allow(non_snake_case)]
pub async fn TrafficMetricsMiddleware(
    State(traffic_stats): State<Arc<TrafficStats>>,
    req: Request,
    next: Next,
) -> Response {
    let started = std::time::Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let operation = classify_traffic_operation(&method, &path);
    let response = next.run(req).await;

    if let Some(operation) = operation {
        let is_error = response.status().is_client_error() || response.status().is_server_error();
        let latency_micros = started.elapsed().as_micros() as u64;

        match operation {
            TrafficOperation::Read => {
                traffic_stats.record_read(0);
                traffic_stats.record_read_outcome(!is_error, latency_micros);
            }
            TrafficOperation::Write => {
                traffic_stats.record_write(0);
                traffic_stats.record_write_outcome(!is_error, latency_micros);
            }
            TrafficOperation::List => {
                traffic_stats.record_list();
                traffic_stats.record_list_outcome(!is_error, latency_micros);
            }
            TrafficOperation::Other => {
                traffic_stats.record_other();
                traffic_stats.record_other_outcome(!is_error, latency_micros);
            }
        }

        if is_error {
            traffic_stats.record_error();
        }

        // N7: Record duration in histogram buckets (seconds) for p50/p95/p99
        let duration_secs = started.elapsed().as_secs_f64();
        traffic_stats.record_request_duration_secs(duration_secs);
    }

    response
}

/// Phase 17.1: 租户中间件
/// 从 X-Tenant-ID header 提取租户 ID 并注入到请求扩展中
#[allow(non_snake_case)]
pub async fn TenantMiddleware(
    State(tenant_state): State<TenantState>,
    mut req: Request,
    next: Next,
) -> Response {
    let tenant_header = req.headers().get(TENANT_HEADER).cloned();
    let tenant_id = TenantState::effective_tenant_id(tenant_header.as_ref());

    req.extensions_mut().insert(tenant_id);
    req.extensions_mut().insert(tenant_state);

    next.run(req).await
}

/// 认证中间件
#[allow(non_snake_case)]
pub async fn AuthMiddleware(
    State(auth_state): State<Arc<RestAuthState>>,
    mut req: Request,
    next: Next,
) -> Response {
    if !auth_state.is_enforced() {
        return next.run(req).await;
    }

    let Some(requirement) = route_requirement(req.method(), req.uri().path()) else {
        return next.run(req).await;
    };

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match auth_state.authorize(req.headers(), requirement) {
        AuthDecision::Granted(identity) => {
            let lease = match auth_state.try_acquire_rate_limit(identity.principal_id) {
                Ok(lease) => lease,
                Err(limit) => {
                    warn!("rate limit exceeded for {} {}", method, path);
                    return rate_limit_response(limit, "API key concurrency limit exceeded");
                }
            };
            auth_state.record_event(
                AuditEventType::AccessGranted,
                Some(identity.principal_id),
                requirement,
                &method,
                &path,
                true,
                req.headers(),
                "authorized",
            );
            req.extensions_mut().insert(identity.principal_id);
            let mut response = next.run(req).await;
            if let Some(lease) = lease {
                apply_rate_limit_headers(&mut response, lease.limit, lease.remaining);
            }
            response
        }
        AuthDecision::MissingCredentials => {
            warn!("missing credentials for {} {}", method, path);
            auth_state.record_event(
                AuditEventType::AuthenticationFailed,
                None,
                requirement,
                &method,
                &path,
                false,
                req.headers(),
                "missing credentials",
            );
            auth_error_response(
                StatusCode::UNAUTHORIZED,
                "Authentication required for protected REST path",
            )
        }
        AuthDecision::InvalidCredentials => {
            warn!("invalid credentials for {} {}", method, path);
            auth_state.record_event(
                AuditEventType::AuthenticationFailed,
                None,
                requirement,
                &method,
                &path,
                false,
                req.headers(),
                "invalid credentials",
            );
            auth_error_response(StatusCode::UNAUTHORIZED, "Invalid API key")
        }
        AuthDecision::Forbidden(identity) => {
            warn!(
                "forbidden {} {} for {}",
                method, path, identity.principal_id
            );
            auth_state.record_event(
                AuditEventType::AccessDenied,
                Some(identity.principal_id),
                requirement,
                &method,
                &path,
                false,
                req.headers(),
                "insufficient permissions",
            );
            auth_error_response(StatusCode::FORBIDDEN, "Insufficient permissions")
        }
        AuthDecision::Internal(message) => {
            warn!(
                "auth middleware internal error on {} {}: {}",
                method, path, message
            );
            auth_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed")
        }
    }
}

fn rate_limit_response(limit: usize, message: &str) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "TOO_MANY_REQUESTS",
            "message": message,
        })),
    )
        .into_response();
    apply_rate_limit_headers(&mut response, limit, 0);
    if let Ok(value) = HeaderValue::from_str(&RETRY_AFTER_SECS.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(RETRY_AFTER_HEADER), value);
    }
    response
}

fn apply_rate_limit_headers(response: &mut Response, limit: usize, remaining: usize) {
    if let Ok(value) = HeaderValue::from_str(&limit.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(RATE_LIMIT_LIMIT_HEADER), value);
    }
    if let Ok(value) = HeaderValue::from_str(&remaining.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(RATE_LIMIT_REMAINING_HEADER), value);
    }
}

fn apply_rate_limit_headers_if_missing(response: &mut Response, limit: usize, remaining: usize) {
    if !response
        .headers()
        .contains_key(HeaderName::from_static(RATE_LIMIT_LIMIT_HEADER))
    {
        apply_rate_limit_headers(response, limit, remaining);
    }
}

fn normalize_rate_limit(limit: usize) -> Option<usize> {
    if limit == 0 {
        None
    } else {
        Some(limit)
    }
}

fn parse_optional_usize_env(key: &str) -> Option<usize> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .and_then(normalize_rate_limit)
}

fn parse_key_list(var_name: &str) -> Vec<String> {
    std::env::var(var_name)
        .ok()
        .map(|value| {
            value
                .split([',', '\n'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_hashed_key_list(var_name: &str) -> Vec<String> {
    parse_key_list(var_name)
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
        .collect()
}

fn audit_log_from_env() -> Arc<AuditLogManager> {
    match std::env::var("EVIF_REST_AUTH_AUDIT_LOG") {
        Ok(path) if !path.trim().is_empty() => Arc::new(
            AuditLogManager::from_file(path.trim())
                .unwrap_or_else(|_| AuditLogManager::from_memory()),
        ),
        _ => Arc::new(AuditLogManager::from_memory()),
    }
}

fn register_key(
    manager: &AuthManager,
    api_keys: &mut HashMap<String, ApiKeyIdentity>,
    key: String,
    admin: bool,
) {
    let principal_id = Uuid::new_v4();
    let identity = ApiKeyIdentity {
        principal: Principal::Service(principal_id),
        principal_id,
    };

    let _ = manager.grant(Capability::new(
        principal_id,
        write_scope_id(),
        Permissions::read_write(),
    ));

    if admin {
        let _ = manager.grant(Capability::new(
            principal_id,
            admin_scope_id(),
            Permissions::all(),
        ));
    }

    api_keys.insert(key, identity);
}

fn register_hashed_key(
    manager: &AuthManager,
    api_keys: &mut HashMap<String, ApiKeyIdentity>,
    key_hash: String,
    admin: bool,
) {
    let principal_id = Uuid::new_v4();
    let identity = ApiKeyIdentity {
        principal: Principal::Service(principal_id),
        principal_id,
    };

    let _ = manager.grant(Capability::new(
        principal_id,
        write_scope_id(),
        Permissions::read_write(),
    ));

    if admin {
        let _ = manager.grant(Capability::new(
            principal_id,
            admin_scope_id(),
            Permissions::all(),
        ));
    }

    api_keys.insert(key_hash, identity);
}

fn sha256_hex(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

/// 从环境变量加载 JWT 验证器配置
fn jwt_validator_from_env() -> Option<JwtValidator> {
    let secret = std::env::var("EVIF_REST_JWT_SECRET").ok()?;

    let mut validator = JwtValidator::with_secret(&secret);

    if let Some(issuer) = std::env::var("EVIF_REST_JWT_ISSUER").ok() {
        validator = validator.with_issuer(&issuer);
    }

    if let Some(audience) = std::env::var("EVIF_REST_JWT_AUDIENCE").ok() {
        validator = validator.with_audience(&audience);
    }

    Some(validator)
}

fn header_value(headers: &HeaderMap, name: impl axum::http::header::AsHeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn resolve_request_identity(headers: &HeaderMap) -> RequestIdentity {
    let request_id = header_value(headers, REQUEST_ID_HEADER)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let correlation_id = header_value(headers, CORRELATION_ID_HEADER)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| request_id.clone());

    RequestIdentity {
        request_id,
        correlation_id,
    }
}

fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    header_value(headers, "x-api-key")
        .or_else(|| header_value(headers, "x-evif-api-key"))
        .or_else(|| {
            header_value(headers, AUTHORIZATION).and_then(|value| {
                value
                    .strip_prefix("Bearer ")
                    .or_else(|| value.strip_prefix("bearer "))
                    .map(str::trim)
            })
        })
}

fn write_requirement() -> RouteRequirement {
    RouteRequirement {
        permission: Permission::Write,
        scope: "write",
        resource_id: write_scope_id(),
    }
}

fn admin_requirement() -> RouteRequirement {
    RouteRequirement {
        permission: Permission::Admin,
        scope: "admin",
        resource_id: admin_scope_id(),
    }
}

fn route_requirement(method: &Method, path: &str) -> Option<RouteRequirement> {
    let is_write_route = (*method == Method::POST
        && matches!(
            path,
            "/api/v1/fs/write"
                | "/api/v1/fs/create"
                | "/api/v1/fs/stream"
                | "/api/v1/fs/chmod"
                | "/api/v1/fs/chown"
                | "/api/v1/files"
                | "/api/v1/directories"
                | "/api/v1/touch"
                | "/api/v1/rename"
                | "/api/v1/memories"
                | "/api/v1/digest"
                | "/api/v1/grep"
                | "/api/v1/copy"
        ))
        || (*method == Method::PUT && matches!(path, "/api/v1/files"))
        || (*method == Method::DELETE
            && matches!(
                path,
                "/api/v1/fs/delete" | "/api/v1/files" | "/api/v1/directories"
            ))
        || (*method == Method::POST && path == "/api/v1/copy/recursive")
        || (*method == Method::POST && path.starts_with("/nodes/create/"))
        || (*method == Method::DELETE && path.starts_with("/nodes/"))
        || path.starts_with("/api/v1/handles")
        || path.starts_with("/api/v1/batch/")
        || path.starts_with("/api/v1/share/")
        || path.starts_with("/api/v1/permissions/")
        || path.starts_with("/api/v1/comments")
        || path == "/api/v1/activities"
        || path == "/api/v1/users"
        // File lock operations require write permission
        || path == "/api/v1/lock"
        || path == "/api/v1/locks"
        // Sync write operations
        || path == "/api/v1/sync/delta"
        || path == "/api/v1/sync/resolve"
        // LLM operations are expensive
        || path == "/api/v1/llm/complete"
        // Memory queries access sensitive data
        || path == "/api/v1/memories/search"
        || path == "/api/v1/memories/query";

    if is_write_route {
        return Some(write_requirement());
    }

    let is_admin_route = (*method == Method::POST
        && matches!(
            path,
            "/api/v1/mount"
                | "/api/v1/unmount"
                | "/api/v1/plugins/load"
                | "/api/v1/plugins/unload"
                | "/api/v1/plugins/wasm/load"
                | "/api/v1/plugins/wasm/reload"
                | "/api/v1/encryption/enable"
                | "/api/v1/encryption/disable"
                | "/api/v1/encryption/rotate"
                | "/api/v1/tenants"
                | "/api/v1/metrics/reset"
                | "/api/v1/cloud/config"
        ))
        || (*method == Method::GET && path == "/api/v1/tenants")
        || (*method == Method::GET
            && path.starts_with("/api/v1/tenants/")
            && path != "/api/v1/tenants/me")
        || (*method == Method::GET
            && matches!(
                path,
                "/api/v1/encryption/status" | "/api/v1/encryption/versions"
            ))
        || (*method == Method::DELETE && path.starts_with("/api/v1/tenants/"))
        || (*method == Method::PATCH
            && path.starts_with("/api/v1/tenants/")
            && path.ends_with("/quota"))
        || (*method == Method::POST
            && path.starts_with("/api/v1/plugins/")
            && path.ends_with("/reload"))
        // GraphQL exposes full data model
        || path == "/api/v1/graphql";

    if is_admin_route {
        return Some(admin_requirement());
    }

    None
}

fn auth_error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({
            "error": status.to_string(),
            "message": message,
        })),
    )
        .into_response()
}

/// Panic catcher middleware — prevents panics from killing Axum worker threads.
/// Catches any panic from the inner handler and returns a 500 response.
pub async fn panic_catcher(request: Request, next: Next) -> Response {
    match tokio::spawn(async move { next.run(request).await }).await {
        Ok(response) => response,
        Err(_) => {
            warn!("Request handler panicked and was caught by panic_catcher");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "INTERNAL_SERVER_ERROR",
                    "message": "Request handler panicked",
                })),
            )
                .into_response()
        }
    }
}

// N8: Timeout middleware — 30s request timeout using tokio::time::timeout.
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub async fn timeout_middleware(request: Request, next: Next) -> Response {
    match tokio::time::timeout(
        std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS),
        next.run(request),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => {
            tracing::info!(timeout_secs = REQUEST_TIMEOUT_SECS, "Request timed out");
            (StatusCode::REQUEST_TIMEOUT, "Request timeout").into_response()
        }
    }
}

// N5: Concurrency limit middleware — limit concurrent requests via semaphore.
static CONCURRENCY_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();
const MAX_CONCURRENT_REQUESTS: usize = 256;

pub async fn concurrency_limit_middleware(request: Request, next: Next) -> Response {
    let semaphore = CONCURRENCY_SEMAPHORE.get_or_init(|| Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let permit = semaphore.acquire().await.expect("semaphore closed");
    let response = next.run(request).await;
    drop(permit);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{middleware, routing::post, Router};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use tokio::net::TcpListener;

    #[test]
    fn test_write_route_classification() {
        assert_eq!(
            route_requirement(&Method::PUT, "/api/v1/files").map(|req| req.permission),
            Some(Permission::Write)
        );
        assert_eq!(
            route_requirement(&Method::POST, "/api/v1/memories").map(|req| req.permission),
            Some(Permission::Write)
        );
        assert_eq!(route_requirement(&Method::GET, "/api/v1/files"), None);
    }

    #[test]
    fn test_admin_route_classification() {
        assert_eq!(
            route_requirement(&Method::POST, "/api/v1/mount").map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::POST, "/api/v1/tenants").map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::PATCH, "/api/v1/tenants/default/quota")
                .map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::GET, "/api/v1/encryption/status").map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::GET, "/api/v1/encryption/versions")
                .map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::POST, "/api/v1/plugins/foo/reload")
                .map(|req| req.permission),
            Some(Permission::Admin)
        );
        assert_eq!(
            route_requirement(&Method::GET, "/api/v1/plugins/foo/status"),
            None
        );
    }

    #[test]
    fn test_extract_api_key_supports_multiple_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test-key".parse().unwrap());
        assert_eq!(extract_api_key(&headers), Some("test-key"));

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer bearer-key".parse().unwrap());
        assert_eq!(extract_api_key(&headers), Some("bearer-key"));
    }

    #[test]
    fn test_auth_state_records_audit_events() {
        let state = RestAuthState::from_api_keys(vec!["write-key".to_string()], vec![]);
        let requirement = write_requirement();
        let headers = HeaderMap::new();

        if let AuthDecision::MissingCredentials = state.authorize(&headers, requirement) {}

        state.record_event(
            AuditEventType::AuthenticationFailed,
            None,
            requirement,
            &Method::PUT,
            "/api/v1/files",
            false,
            &headers,
            "missing credentials",
        );

        let events = state.audit_events();
        assert_eq!(events.len(), 1);
        assert!(events[0].details.contains("/api/v1/files"));
    }

    #[test]
    fn test_resolve_request_identity_generates_defaults() {
        let headers = HeaderMap::new();
        let identity = resolve_request_identity(&headers);

        assert!(Uuid::parse_str(&identity.request_id).is_ok());
        assert_eq!(identity.correlation_id, identity.request_id);
    }

    #[test]
    fn test_resolve_request_identity_preserves_client_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID_HEADER, "req-123".parse().unwrap());
        headers.insert(CORRELATION_ID_HEADER, "corr-456".parse().unwrap());

        let identity = resolve_request_identity(&headers);
        assert_eq!(identity.request_id, "req-123");
        assert_eq!(identity.correlation_id, "corr-456");
    }

    #[test]
    fn test_classify_traffic_operation() {
        assert_eq!(
            classify_traffic_operation(&Method::GET, "/api/v1/files"),
            Some(TrafficOperation::Read)
        );
        assert_eq!(
            classify_traffic_operation(&Method::GET, "/api/v1/directories"),
            Some(TrafficOperation::List)
        );
        assert_eq!(
            classify_traffic_operation(&Method::PUT, "/api/v1/files"),
            Some(TrafficOperation::Write)
        );
        assert_eq!(
            classify_traffic_operation(&Method::POST, "/api/v1/metrics/reset"),
            None
        );
    }

    async fn spawn_server(app: Router) -> (String, reqwest::Client) {
        let listener = TcpListener::bind("127.0.0.1:0").await
            .expect("Failed to bind TCP port - check macOS sandbox restrictions");
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .expect("serve");
        });

        let base = format!("http://127.0.0.1:{}", port);
        let client = reqwest::Client::new();
        for _ in 0..60 {
            if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
                if res.status().is_success() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        (base, client)
    }

    #[tokio::test]
    async fn test_api_key_rate_limit_headers_are_present() {
        async fn handler() -> &'static str {
            "ok"
        }

        let auth_state = Arc::new(RestAuthState::from_api_keys_with_concurrency_limit(
            vec!["write-key".to_string()],
            vec![],
            2,
        ));
        let app = Router::new()
            .route("/api/v1/files", post(handler))
            .route("/api/v1/health", axum::routing::get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth_state, AuthMiddleware));

        let (base, client) = spawn_server(app).await;
        let response = client
            .post(format!("{}/api/v1/files", base))
            .header("x-api-key", "write-key")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("x-ratelimit-limit")
                .and_then(|value| value.to_str().ok()),
            Some("2")
        );
        assert_eq!(
            response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );
    }

    #[tokio::test]
    async fn test_api_key_rate_limit_rejects_second_inflight_request() {
        async fn slow_handler(State(started): State<Arc<AtomicBool>>) -> &'static str {
            started.store(true, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(250)).await;
            "ok"
        }

        let auth_state = Arc::new(RestAuthState::from_api_keys_with_concurrency_limit(
            vec!["write-key".to_string()],
            vec![],
            1,
        ));
        let started = Arc::new(AtomicBool::new(false));
        let app = Router::new()
            .route("/api/v1/files", post(slow_handler))
            .route("/api/v1/health", axum::routing::get(|| async { "ok" }))
            .with_state(started.clone())
            .layer(middleware::from_fn_with_state(auth_state, AuthMiddleware));

        let (base, client) = spawn_server(app).await;
        let in_flight_client = client.clone();
        let in_flight_base = base.clone();
        let first = tokio::spawn(async move {
            in_flight_client
                .post(format!("{}/api/v1/files", in_flight_base))
                .header("x-api-key", "write-key")
                .send()
                .await
                .unwrap()
        });

        for _ in 0..20 {
            if started.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let second = client
            .post(format!("{}/api/v1/files", base))
            .header("x-api-key", "write-key")
            .send()
            .await
            .unwrap();

        assert_eq!(second.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            second
                .headers()
                .get("x-ratelimit-limit")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );
        assert_eq!(
            second
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|value| value.to_str().ok()),
            Some("0")
        );
        assert_eq!(
            second
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );

        let first = first.await.unwrap();
        assert_eq!(first.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ip_rate_limit_isolated_per_client_ip() {
        async fn slow_handler(State(started): State<Arc<AtomicBool>>) -> &'static str {
            started.store(true, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(250)).await;
            "ok"
        }

        let started = Arc::new(AtomicBool::new(false));
        let app = Router::new()
            .route("/api/v1/files", post(slow_handler))
            .route("/api/v1/health", axum::routing::get(|| async { "ok" }))
            .with_state(started.clone())
            .layer(middleware::from_fn_with_state(
                Arc::new(IpRateLimitState::new(Some(1))),
                IpRateLimitMiddleware,
            ));

        let (base, client) = spawn_server(app).await;
        let first_client = client.clone();
        let first_base = base.clone();
        let first = tokio::spawn(async move {
            first_client
                .post(format!("{}/api/v1/files", first_base))
                .header("x-real-ip", "203.0.113.10")
                .send()
                .await
                .unwrap()
        });

        for _ in 0..20 {
            if started.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let same_ip = client
            .post(format!("{}/api/v1/files", base))
            .header("x-real-ip", "203.0.113.10")
            .send()
            .await
            .unwrap();
        assert_eq!(same_ip.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            same_ip
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );
        let same_ip_json: serde_json::Value = same_ip.json().await.unwrap();
        assert_eq!(same_ip_json["message"], "IP concurrency limit exceeded");

        let different_ip = client
            .post(format!("{}/api/v1/files", base))
            .header("x-real-ip", "203.0.113.11")
            .send()
            .await
            .unwrap();
        assert_eq!(different_ip.status(), reqwest::StatusCode::OK);

        let first = first.await.unwrap();
        assert_eq!(first.status(), reqwest::StatusCode::OK);
    }
}
