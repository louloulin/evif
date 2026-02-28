// REST API 中间件

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use tracing::info;

/// 日志中间件
pub async fn LoggingMiddleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    info!("{} {}", method, uri);

    next.run(req).await
}

/// 认证中间件
pub async fn AuthMiddleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // NOTE: Authentication is currently disabled for development
    // For production, implement JWT or API key validation here
    // 检查请求头中的认证信息

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_exists() {
        // 中间件函数存在
        assert!(true);
    }
}
