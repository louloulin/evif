// 响应类型定义

use serde::{Deserialize, Serialize};
use evif_graph::{Node, NodeId};

/// 响应状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// 成功
    Success,

    /// 部分成功
    PartialSuccess,

    /// 失败
    Failed,

    /// 超时
    Timeout,

    /// 未授权
    Unauthorized,
}

/// 响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseKind {
    /// Pong 响应
    Pong,

    /// 节点数据
    Node { node: Option<Node> },

    /// 创建结果
    Created { id: NodeId },

    /// 更新确认
    Updated,

    /// 删除确认
    Deleted,

    /// 查询结果
    QueryResult { ids: Vec<NodeId>, count: usize },

    /// 子节点列表
    Children { ids: Vec<NodeId> },

    /// 文件操作结果
    FileResult {
        handle: Option<u64>,
        size: Option<u64>,
        data: Option<Vec<u8>>,
    },

    /// 批量操作结果
    Batch { results: Vec<Response> },

    /// 流式响应
    Stream { stream_id: u64 },
}

/// 响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// 对应的请求 ID
    pub request_id: u64,

    /// 响应状态
    pub status: ResponseStatus,

    /// 响应种类
    pub kind: ResponseKind,

    /// 时间戳
    pub timestamp: u64,

    /// 错误信息
    pub error: Option<String>,
}

impl Response {
    /// 创建新响应
    pub fn new(request_id: u64, status: ResponseStatus, kind: ResponseKind) -> Self {
        Response {
            request_id,
            status,
            kind,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            error: None,
        }
    }

    /// 创建成功响应
    pub fn success(request_id: u64, kind: ResponseKind) -> Self {
        Self::new(request_id, ResponseStatus::Success, kind)
    }

    /// 创建错误响应
    pub fn error(request_id: u64, error: String) -> Self {
        let mut response = Self::new(request_id, ResponseStatus::Failed, ResponseKind::Pong);
        response.error = Some(error);
        response
    }

    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        matches!(self.status, ResponseStatus::Success | ResponseStatus::PartialSuccess)
    }

    /// 获取错误信息
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// 估算大小
    pub fn estimated_size(&self) -> usize {
        match &self.kind {
            ResponseKind::Pong => 32,
            ResponseKind::Node { node } => {
                node.as_ref().map(|_n| 256).unwrap_or(64)
            }
            ResponseKind::Created { .. } => 64,
            ResponseKind::Updated => 32,
            ResponseKind::Deleted => 32,
            ResponseKind::QueryResult { ids, .. } => 64 + ids.len() * 16,
            ResponseKind::Children { ids } => 64 + ids.len() * 16,
            ResponseKind::FileResult { data, .. } => {
                64 + data.as_ref().map(|d| d.len()).unwrap_or(0)
            }
            ResponseKind::Batch { results } => {
                results.iter().map(|r| r.estimated_size()).sum()
            }
            ResponseKind::Stream { .. } => 32,
        }
    }
}

/// 便捷方法：创建各种响应
impl Response {
    pub fn pong(request_id: u64) -> Self {
        Self::success(request_id, ResponseKind::Pong)
    }

    pub fn node(request_id: u64, node: Option<Node>) -> Self {
        Self::success(request_id, ResponseKind::Node { node })
    }

    pub fn created(request_id: u64, id: NodeId) -> Self {
        Self::success(request_id, ResponseKind::Created { id })
    }

    pub fn deleted(request_id: u64) -> Self {
        Self::success(request_id, ResponseKind::Deleted)
    }

    pub fn query_result(request_id: u64, ids: Vec<NodeId>, count: usize) -> Self {
        Self::success(request_id, ResponseKind::QueryResult { ids, count })
    }

    pub fn children(request_id: u64, ids: Vec<NodeId>) -> Self {
        Self::success(request_id, ResponseKind::Children { ids })
    }

    pub fn file_result(
        request_id: u64,
        handle: Option<u64>,
        size: Option<u64>,
        data: Option<Vec<u8>>,
    ) -> Self {
        Self::success(request_id, ResponseKind::FileResult {
            handle,
            size,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        let req_id = 123;
        let resp = Response::pong(req_id);

        assert_eq!(resp.request_id, req_id);
        assert!(resp.is_success());
        assert!(resp.is_success());
    }

    #[test]
    fn test_error_response() {
        let req_id = 123;
        let resp = Response::error(req_id, "Test error".to_string());

        assert_eq!(resp.request_id, req_id);
        assert!(!resp.is_success());
        assert_eq!(resp.get_error(), Some("Test error"));
    }

    #[test]
    fn test_node_response() {
        let req_id = 123;
        let node = Some(Node::new(evif_graph::NodeType::File, "test.txt"));
        let resp = Response::node(req_id, node);

        assert!(resp.is_success());
        match resp.kind {
            ResponseKind::Node { node: Some(n) } => {
                assert_eq!(n.name, "test.txt");
            }
            _ => panic!("Expected node response"),
        }
    }

    #[test]
    fn test_response_status() {
        let resp = Response::new(
            123,
            ResponseStatus::Success,
            ResponseKind::Pong,
        );

        assert_eq!(resp.status, ResponseStatus::Success);
        assert!(resp.is_success());
    }
}
