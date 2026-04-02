// Phase 17.4: GraphQL API
//
// 提供 GraphQL API 端点

use async_graphql::{Context, EmptySubscription, Object, Schema, SimpleObject};
use async_graphql_axum::GraphQL;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// GraphQL state
#[derive(Clone)]
pub struct GraphQLState {
    inner: Arc<RwLock<GraphQLInner>>,
}

struct GraphQLInner {
    enabled: bool,
}

/// File info for GraphQL
#[derive(SimpleObject, Clone, Serialize, Deserialize)]
pub struct GqlFileInfo {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<String>,
}

/// Query root for GraphQL
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get server status
    async fn status(&self) -> ServerStatus {
        ServerStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: "running".to_string(),
        }
    }

    /// Get health check
    async fn health(&self) -> bool {
        true
    }
}

/// Mutation root for GraphQL
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Echo test mutation
    async fn echo(&self, message: String) -> String {
        message
    }
}

/// Server status
#[derive(SimpleObject)]
pub struct ServerStatus {
    pub version: String,
    pub status: String,
}

/// GraphQL schema
pub type EvifSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

impl GraphQLState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GraphQLInner { enabled: true })),
        }
    }

    pub fn schema() -> EvifSchema {
        Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish()
    }
}

impl Default for GraphQLState {
    fn default() -> Self {
        Self::new()
    }
}

/// GraphQL handlers
pub struct GraphQLHandlers;

impl GraphQLHandlers {
    /// GraphQL endpoint - POST /api/v1/graphql
    pub async fn handler(
        State(schema): State<EvifSchema>,
        req: Json<async_graphql::Request>,
    ) -> impl IntoResponse {
        let response = schema.execute(req.0).await;
        Json(async_graphql::Response::from(response))
    }

    /// GraphQL IDE (GraphiQL) - GET /api/v1/graphql/graphiql
    pub async fn graphiql() -> impl IntoResponse {
        axum::response::Html(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>EVIF GraphQL</title>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body style="margin: 0;">
    <div id="graphiql" style="height: 100vh;"></div>
    <script crossorigin src="https://unpkg.com/react/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom/umd/react-dom.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/graphiql/graphiql.min.js"></script>
    <script>
        const fetcher = GraphiQL.createFetcher({ url: '/api/v1/graphql' });
        ReactDOM.render(React.createElement(GraphiQL, { fetcher }), document.getElementById('graphiql'));
    </script>
</body>
</html>"#,
        )
    }
}
