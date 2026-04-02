// Phase 17.4: GraphQL API
//
// 提供 GraphQL API 端点

use async_graphql::{EmptySubscription, Object, Schema, SimpleObject};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};

/// GraphQL state
#[derive(Clone)]
pub struct GraphQLState;

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
        Self
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
        Json(schema.execute(req.0).await)
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
