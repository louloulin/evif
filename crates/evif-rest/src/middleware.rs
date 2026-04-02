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
    Permission, Permissions, Principal,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::tenant_handlers::{TenantState, TENANT_HEADER};
use crate::TrafficStats;

const REQUEST_ID_HEADER: &str = "x-request-id";
const CORRELATION_ID_HEADER: &str = "x-correlation-id";

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
    audit_log: Arc<AuditLogManager>,
    enforce: bool,
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

impl RestAuthState {
    /// 从环境变量构造 REST 认证配置。
    ///
    /// - `EVIF_REST_AUTH_MODE=disabled|off|false` 可关闭鉴权（默认开启）
    /// - `EVIF_REST_WRITE_API_KEYS=key1,key2`
    /// - `EVIF_REST_ADMIN_API_KEYS=key3,key4`
    /// - `EVIF_REST_AUTH_AUDIT_LOG=/path/to/evif-audit.log`
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
        let audit_log = audit_log_from_env();

        Self::new(write_keys, admin_keys, audit_log, enforce)
    }

    pub fn disabled() -> Self {
        Self::new(
            vec![],
            vec![],
            Arc::new(AuditLogManager::from_memory()),
            false,
        )
    }

    pub fn from_api_keys(
        write_keys: impl IntoIterator<Item = String>,
        admin_keys: impl IntoIterator<Item = String>,
    ) -> Self {
        Self::new(
            write_keys.into_iter().collect(),
            admin_keys.into_iter().collect(),
            Arc::new(AuditLogManager::from_memory()),
            true,
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
        audit_log: Arc<AuditLogManager>,
        enforce: bool,
    ) -> Self {
        let manager = AuthManager::with_policy(AuthPolicy::Strict);
        let mut api_keys = HashMap::new();

        if enforce {
            for key in write_keys {
                register_key(&manager, &mut api_keys, key, false);
            }

            for key in admin_keys {
                register_key(&manager, &mut api_keys, key, true);
            }
        }

        Self {
            inner: Arc::new(RestAuthInner {
                manager,
                api_keys,
                audit_log,
                enforce,
            }),
        }
    }

    fn is_enforced(&self) -> bool {
        self.inner.enforce
    }

    fn authorize(&self, headers: &HeaderMap, requirement: RouteRequirement) -> AuthDecision {
        let Some(api_key) = extract_api_key(headers) else {
            return AuthDecision::MissingCredentials;
        };

        let Some(identity) = self.inner.api_keys.get(api_key).cloned() else {
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

/// Validates and sanitizes a filesystem path to prevent path traversal attacks.
///
/// Returns the sanitized path on success. Rejects paths containing:
/// - `..` (directory traversal)
/// - Null bytes (`\x00`)
/// - Double slashes (`//`)
/// - Paths exceeding max length (4096 bytes)
pub fn validate_path(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    if path.len() > 4096 {
        return Err(format!("Path too long: {} bytes (max 4096)", path.len()));
    }

    // Reject null bytes
    if path.contains('\0') {
        return Err("Path contains null bytes".to_string());
    }

    // Reject path traversal attempts
    let segments: Vec<&str> = path.split('/').collect();
    for seg in &segments {
        if *seg == ".." {
            return Err("Path traversal not allowed (..)".to_string());
        }
    }

    // Normalize: remove double slashes. Ensure leading /.
    let normalized = {
        let mut result = String::with_capacity(path.len());
        let mut last_was_slash = false;
        for ch in path.chars() {
            if ch == '/' {
                if last_was_slash {
                    continue;
                }
                last_was_slash = true;
            } else {
                last_was_slash = false;
            }
            result.push(ch);
        }
        result
    };

    // Ensure path starts with /
    if !normalized.starts_with('/') {
        return Ok(format!("/{}", normalized));
    }

    Ok(normalized)
}

/// Path validation middleware - sanitizes path query parameters.
/// Blocks requests with path traversal patterns, null bytes, or oversized paths.
#[allow(non_snake_case)]
pub async fn PathValidationMiddleware(request: Request, next: Next) -> Response {
    // Check for path parameter in query string
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some(path_val) = pair.strip_prefix("path=") {
                match validate_path(path_val) {
                    Ok(_) => {}
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": "INVALID_PATH",
                                "message": e,
                            })),
                        )
                        .into_response();
                    }
                }
            }
        }
    }
    next.run(request).await
}

/// 日志中间件
#[allow(non_snake_case)]
pub async fn LoggingMiddleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    info!("{} {}", method, uri);

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
            next.run(req).await
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

fn header_value(
    headers: &HeaderMap,
    name: impl axum::http::header::AsHeaderName,
) -> Option<&str> {
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
                | "/api/v1/files"
                | "/api/v1/directories"
                | "/api/v1/touch"
                | "/api/v1/rename"
                | "/api/v1/memories"
        ))
        || (*method == Method::PUT && matches!(path, "/api/v1/files"))
        || (*method == Method::DELETE
            && matches!(
                path,
                "/api/v1/fs/delete" | "/api/v1/files" | "/api/v1/directories"
            ))
        || (*method == Method::POST && path.starts_with("/nodes/create/"))
        || (*method == Method::DELETE && path.starts_with("/nodes/"))
        || path.starts_with("/api/v1/handles")
        || path.starts_with("/api/v1/batch/")
        || path.starts_with("/api/v1/share/")
        || path.starts_with("/api/v1/permissions/")
        || path.starts_with("/api/v1/comments")
        || path == "/api/v1/activities"
        || path == "/api/v1/users";

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
                | "/api/v1/metrics/reset"
        ))
        || (*method == Method::POST
            && path.starts_with("/api/v1/plugins/")
            && path.ends_with("/reload"));

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
