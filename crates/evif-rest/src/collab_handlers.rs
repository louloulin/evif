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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collab_state_default() {
        let state = CollabState::default();
        assert!(state.shares.try_read().is_ok());
        assert!(state.comments.try_read().is_ok());
        assert!(state.activities.try_read().is_ok());
        assert!(state.permissions.try_read().is_ok());
    }

    #[test]
    fn test_share_record_serialization() {
        let record = ShareRecord {
            id: "share-1".to_string(),
            file_id: "file-1".to_string(),
            file_path: "/path/to/file".to_string(),
            file_name: "file.txt".to_string(),
            access_url: "https://example.com/share/abc".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: Some("2024-12-31T23:59:59Z".to_string()),
            permissions: vec![],
            access_count: 5,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("share-1"));
        assert!(json.contains("file-1"));
        assert!(json.contains("file.txt"));
    }

    #[test]
    fn test_share_permission_record() {
        let record = SharePermissionRecord {
            user_id: "user-1".to_string(),
            user_name: "John Doe".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        assert_eq!(record.user_id, "user-1");
        assert_eq!(record.permissions.len(), 2);
    }

    #[test]
    fn test_comment_record_serialization() {
        let record = CommentRecord {
            id: "comment-1".to_string(),
            file_id: "file-1".to_string(),
            file_path: "/path/to/file".to_string(),
            content: "This is a comment".to_string(),
            author: "John Doe".to_string(),
            author_id: "user-1".to_string(),
            line_number: Some(42),
            column: Some(10),
            reply_to: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
            resolved: false,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("comment-1"));
        assert!(json.contains("This is a comment"));
        assert!(json.contains("42")); // line_number
    }

    #[test]
    fn test_activity_record_serialization() {
        let record = ActivityRecord {
            id: "activity-1".to_string(),
            activity_type: "edit".to_string(),
            file_id: "file-1".to_string(),
            file_path: "/path/to/file".to_string(),
            file_name: "file.txt".to_string(),
            description: "Edited file.txt".to_string(),
            user_id: "user-1".to_string(),
            user_name: "John Doe".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("activity-1"));
        assert!(json.contains("edit"));
    }

    #[test]
    fn test_create_share_request_deserialization() {
        let json = r#"{
            "file_id": "file-1",
            "file_path": "/path/to/file",
            "file_name": "file.txt",
            "access_type": "public",
            "permissions": [],
            "expires_at": "2024-12-31T23:59:59Z"
        }"#;

        let request: CreateShareRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.file_id, "file-1");
        assert_eq!(request.file_path, "/path/to/file");
        assert_eq!(request.access_type, "public");
    }

    #[test]
    fn test_create_share_request_defaults() {
        let json = r#"{
            "file_id": "file-1",
            "file_path": "/path/to/file",
            "file_name": "file.txt"
        }"#;

        let request: CreateShareRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.file_id, "file-1");
        assert_eq!(request.access_type, "");
        assert!(request.expires_at.is_none());
    }

    #[test]
    fn test_share_response_serialization() {
        let response = ShareResponse {
            id: "share-1".to_string(),
            access_url: "https://example.com/share/abc".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            expires_at: Some("2024-12-31T23:59:59Z".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("share-1"));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn test_list_shares_response_serialization() {
        let response = ListSharesResponse {
            shares: vec![ShareListItem {
                id: "share-1".to_string(),
                file_id: "file-1".to_string(),
                file_name: "file.txt".to_string(),
                file_path: "/path/to/file".to_string(),
                created_by: "user-1".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: None,
                access_url: "https://example.com/share/abc".to_string(),
                permissions: vec![],
                access_count: 5,
            }],
            total: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total"));
        assert!(json.contains("1"));
    }

    #[test]
    fn test_comment_list_item_serialization() {
        let item = CommentListItem {
            id: "comment-1".to_string(),
            file_id: "file-1".to_string(),
            file_path: "/path/to/file".to_string(),
            content: "This is a comment".to_string(),
            author: "John Doe".to_string(),
            author_id: "user-1".to_string(),
            line_number: Some(42),
            column: Some(10),
            reply_to: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: None,
            resolved: false,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("comment-1"));
    }

    #[test]
    fn test_activity_list_item_serialization() {
        let item = ActivityListItem {
            id: "activity-1".to_string(),
            r#type: "edit".to_string(),
            file_id: "file-1".to_string(),
            file_path: "/path/to/file".to_string(),
            file_name: "file.txt".to_string(),
            description: "Edited file.txt".to_string(),
            user_id: "user-1".to_string(),
            user_name: "John Doe".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("activity-1"));
        assert!(json.contains("edit"));
    }

    #[test]
    fn test_user_item_serialization() {
        let user = UserItem {
            id: "user-1".to_string(),
            name: "John Doe".to_string(),
            email: Some("john@example.com".to_string()),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("user-1"));
        assert!(json.contains("John Doe"));
    }

    #[test]
    fn test_users_response_serialization() {
        let response = UsersResponse {
            users: vec![
                UserItem {
                    id: "user-1".to_string(),
                    name: "John Doe".to_string(),
                    email: None,
                },
                UserItem {
                    id: "user-2".to_string(),
                    name: "Jane Doe".to_string(),
                    email: Some("jane@example.com".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("users"));
        assert!(json.contains("John Doe"));
        assert!(json.contains("Jane Doe"));
    }
}
