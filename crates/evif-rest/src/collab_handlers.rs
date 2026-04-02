// Phase 9.3: 协作 API 处理器（分享/评论/权限/活动）
// 内存存储，与 evif-web collaboration 组件对接

use crate::{RestError, RestResult};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 协作状态（内存存储）
#[derive(Clone, Default)]
pub struct CollabState {
    pub shares: Arc<RwLock<HashMap<String, ShareRecord>>>,
    pub comments: Arc<RwLock<HashMap<String, CommentRecord>>>,
    pub activities: Arc<RwLock<Vec<ActivityRecord>>>,
    pub permissions: Arc<RwLock<HashMap<String, Vec<SharePermissionRecord>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRecord {
    pub id: String,
    pub file_id: String,
    pub file_path: String,
    pub file_name: String,
    pub access_url: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub permissions: Vec<SharePermissionRecord>,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharePermissionRecord {
    pub user_id: String,
    pub user_name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentRecord {
    pub id: String,
    pub file_id: String,
    pub file_path: String,
    pub content: String,
    pub author: String,
    pub author_id: String,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub reply_to: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub id: String,
    pub activity_type: String,
    pub file_id: String,
    pub file_path: String,
    pub file_name: String,
    pub description: String,
    pub user_id: String,
    pub user_name: String,
    pub timestamp: String,
}

// ---------- 请求/响应类型（与前端 collaboration-api 对齐）----------

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub file_id: String,
    pub file_path: String,
    pub file_name: String,
    #[serde(default)]
    pub access_type: String,
    #[serde(default)]
    pub permissions: Vec<serde_json::Value>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShareResponse {
    pub id: String,
    pub access_url: String,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListSharesResponse {
    pub shares: Vec<ShareListItem>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ShareListItem {
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_path: String,
    pub created_by: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub access_url: String,
    pub permissions: Vec<SharePermissionRecord>,
    pub access_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct RevokeShareRequest {
    pub share_id: String,
}

#[derive(Debug, Serialize)]
pub struct RevokeShareResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPermissionsRequest {
    pub file_path: String,
    pub permissions: Vec<SharePermissionRecord>,
}

#[derive(Debug, Serialize)]
pub struct SetPermissionsResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub file_id: String,
    pub file_path: String,
    pub content: String,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListCommentsResponse {
    pub comments: Vec<CommentListItem>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct CommentListItem {
    pub id: String,
    pub file_id: String,
    pub file_path: String,
    pub content: String,
    pub author: String,
    pub author_id: String,
    pub line_number: Option<u32>,
    pub column: Option<u32>,
    pub reply_to: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub resolved: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ResolveCommentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteCommentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ActivitiesResponse {
    pub activities: Vec<ActivityListItem>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ActivityListItem {
    pub id: String,
    pub r#type: String,
    pub file_id: String,
    pub file_path: String,
    pub file_name: String,
    pub description: String,
    pub user_id: String,
    pub user_name: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<UserItem>,
}

#[derive(Debug, Serialize)]
pub struct UserItem {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
}

pub struct CollabHandlers;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl CollabHandlers {
    pub async fn create_share(
        State(state): State<CollabState>,
        Json(req): Json<CreateShareRequest>,
    ) -> RestResult<Json<ShareResponse>> {
        let id = new_id();
        let access_url = format!("/share/{}", id);
        let created_at = now_iso();
        let expires_at = req.expires_at.clone();
        let _access_type = req.access_type;
        let permissions: Vec<SharePermissionRecord> = req
            .permissions
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .collect();
        let record = ShareRecord {
            id: id.clone(),
            file_id: req.file_id,
            file_path: req.file_path.clone(),
            file_name: req.file_name,
            access_url: access_url.clone(),
            created_at: created_at.clone(),
            expires_at: expires_at.clone(),
            permissions,
            access_count: 0,
        };
        state.shares.write().await.insert(id.clone(), record);
        Ok(Json(ShareResponse {
            id,
            access_url,
            created_at,
            expires_at,
        }))
    }

    pub async fn list_shares(
        State(state): State<CollabState>,
        Query(params): Query<HashMap<String, String>>,
    ) -> RestResult<Json<ListSharesResponse>> {
        let file_id = params.get("fileId").cloned();
        let shares = state.shares.read().await;
        let list: Vec<ShareListItem> = shares
            .values()
            .filter(|s| match file_id.as_ref() {
                Some(file_id) => *file_id == s.file_id,
                None => true,
            })
            .map(|s| ShareListItem {
                id: s.id.clone(),
                file_id: s.file_id.clone(),
                file_name: s.file_name.clone(),
                file_path: s.file_path.clone(),
                created_by: "system".to_string(),
                created_at: s.created_at.clone(),
                expires_at: s.expires_at.clone(),
                access_url: s.access_url.clone(),
                permissions: s.permissions.clone(),
                access_count: s.access_count,
            })
            .collect();
        let total = list.len();
        Ok(Json(ListSharesResponse {
            shares: list,
            total,
        }))
    }

    pub async fn revoke_share(
        State(state): State<CollabState>,
        Json(req): Json<RevokeShareRequest>,
    ) -> RestResult<Json<RevokeShareResponse>> {
        let mut shares = state.shares.write().await;
        if shares.remove(&req.share_id).is_some() {
            Ok(Json(RevokeShareResponse {
                success: true,
                message: "Revoked".to_string(),
            }))
        } else {
            Ok(Json(RevokeShareResponse {
                success: false,
                message: "Share not found".to_string(),
            }))
        }
    }

    pub async fn set_permissions(
        State(state): State<CollabState>,
        Json(req): Json<SetPermissionsRequest>,
    ) -> RestResult<Json<SetPermissionsResponse>> {
        state
            .permissions
            .write()
            .await
            .insert(req.file_path, req.permissions);
        Ok(Json(SetPermissionsResponse {
            success: true,
            message: "OK".to_string(),
        }))
    }

    pub async fn get_permissions(
        State(state): State<CollabState>,
        Query(params): Query<HashMap<String, String>>,
    ) -> RestResult<Json<Vec<SharePermissionRecord>>> {
        let path = params.get("path").cloned().unwrap_or_default();
        let perms = state.permissions.read().await;
        let list = perms.get(&path).cloned().unwrap_or_default();
        Ok(Json(list))
    }

    pub async fn list_comments(
        State(state): State<CollabState>,
        Query(params): Query<HashMap<String, String>>,
    ) -> RestResult<Json<ListCommentsResponse>> {
        let path = params.get("path").cloned().unwrap_or_default();
        let comments = state.comments.read().await;
        let list: Vec<CommentListItem> = comments
            .values()
            .filter(|c| c.file_path == path)
            .map(|c| CommentListItem {
                id: c.id.clone(),
                file_id: c.file_id.clone(),
                file_path: c.file_path.clone(),
                content: c.content.clone(),
                author: c.author.clone(),
                author_id: c.author_id.clone(),
                line_number: c.line_number,
                column: c.column,
                reply_to: c.reply_to.clone(),
                created_at: c.created_at.clone(),
                updated_at: c.updated_at.clone(),
                resolved: c.resolved,
            })
            .collect();
        let total = list.len();
        Ok(Json(ListCommentsResponse {
            comments: list,
            total,
        }))
    }

    pub async fn add_comment(
        State(state): State<CollabState>,
        Json(req): Json<CreateCommentRequest>,
    ) -> RestResult<Json<CommentResponse>> {
        let id = new_id();
        let created_at = now_iso();
        let record = CommentRecord {
            id: id.clone(),
            file_id: req.file_id,
            file_path: req.file_path,
            content: req.content,
            author: "user".to_string(),
            author_id: "user".to_string(),
            line_number: req.line_number,
            column: req.column,
            reply_to: req.reply_to,
            created_at: created_at.clone(),
            updated_at: None,
            resolved: false,
        };
        state.comments.write().await.insert(id.clone(), record);
        Ok(Json(CommentResponse { id, created_at }))
    }

    pub async fn update_comment(
        State(state): State<CollabState>,
        Path(id): Path<String>,
        Json(req): Json<UpdateCommentRequest>,
    ) -> RestResult<Json<CommentResponse>> {
        let updated_at = now_iso();
        let mut comments = state.comments.write().await;
        if let Some(c) = comments.get_mut(&id) {
            c.content = req.content;
            c.updated_at = Some(updated_at.clone());
            return Ok(Json(CommentResponse {
                id,
                created_at: c.created_at.clone(),
            }));
        }
        Err(RestError::NotFound(format!("Comment {}", id)))
    }

    pub async fn resolve_comment(
        State(state): State<CollabState>,
        Path(id): Path<String>,
    ) -> RestResult<Json<ResolveCommentResponse>> {
        let mut comments = state.comments.write().await;
        if let Some(c) = comments.get_mut(&id) {
            c.resolved = true;
            return Ok(Json(ResolveCommentResponse {
                success: true,
                message: "Resolved".to_string(),
            }));
        }
        Err(RestError::NotFound(format!("Comment {}", id)))
    }

    pub async fn delete_comment(
        State(state): State<CollabState>,
        Path(id): Path<String>,
    ) -> RestResult<Json<DeleteCommentResponse>> {
        if state.comments.write().await.remove(&id).is_some() {
            Ok(Json(DeleteCommentResponse {
                success: true,
                message: "Deleted".to_string(),
            }))
        } else {
            Err(RestError::NotFound(format!("Comment {}", id)))
        }
    }

    pub async fn get_activities(
        State(state): State<CollabState>,
        Query(params): Query<HashMap<String, String>>,
    ) -> RestResult<Json<ActivitiesResponse>> {
        let path = params.get("path").cloned();
        let limit = params
            .get("limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(50);
        let offset = params
            .get("offset")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let activities = state.activities.read().await;
        let filtered: Vec<&ActivityRecord> = activities
            .iter()
            .filter(|a| match path.as_ref() {
                Some(path) => a.file_path == *path,
                None => true,
            })
            .skip(offset)
            .take(limit)
            .collect();
        let list: Vec<ActivityListItem> = filtered
            .iter()
            .map(|a| ActivityListItem {
                id: a.id.clone(),
                r#type: a.activity_type.clone(),
                file_id: a.file_id.clone(),
                file_path: a.file_path.clone(),
                file_name: a.file_name.clone(),
                description: a.description.clone(),
                user_id: a.user_id.clone(),
                user_name: a.user_name.clone(),
                timestamp: a.timestamp.clone(),
            })
            .collect();
        let total = activities.len();
        Ok(Json(ActivitiesResponse {
            activities: list,
            total,
        }))
    }

    pub async fn list_users(
        Query(params): Query<HashMap<String, String>>,
    ) -> RestResult<Json<UsersResponse>> {
        let _query = params.get("query").cloned();
        Ok(Json(UsersResponse {
            users: vec![UserItem {
                id: "user".to_string(),
                name: "User".to_string(),
                email: None,
            }],
        }))
    }
}
