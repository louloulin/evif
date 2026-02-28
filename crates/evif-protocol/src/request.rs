// 请求类型定义

use serde::{Deserialize, Serialize};
use evif_graph::{NodeId, NodeType};
use std::path::PathBuf;

/// 请求类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestKind {
    /// Ping 请求
    Ping,

    /// 获取节点
    GetNode { id: NodeId },

    /// 创建节点
    CreateNode {
        node_type: NodeType,
        name: String,
        parent_id: Option<NodeId>,
    },

    /// 更新节点
    UpdateNode { id: NodeId },

    /// 删除节点
    DeleteNode { id: NodeId },

    /// 查询图
    QueryGraph { query: String },

    /// 获取子节点
    GetChildren { id: NodeId },

    /// 文件操作
    FileOperation {
        path: PathBuf,
        operation: FileOperation,
    },

    /// 批量操作
    Batch { requests: Vec<Request> },

    /// 流式操作
    Stream { stream_id: u64 },
}

/// 文件操作类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileOperation {
    Open { flags: u32 },
    Close { handle: u64 },
    Read { handle: u64, offset: u64, size: u64 },
    Write { handle: u64, offset: u64 },
    Seek { handle: u64, position: SeekPosition },
    Create { mode: u32 },
    Unlink,
    Rename { new_path: PathBuf },
    Mkdir { mode: u32 },
    Rmdir,
    Readdir,
}

/// 定位位置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeekPosition {
    Start(u64),
    End(i64),
    Current(i64),
}

/// 请求消息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// 请求 ID（用于匹配响应）
    pub id: u64,

    /// 请求种类
    pub kind: RequestKind,

    /// 时间戳
    pub timestamp: u64,

    /// 超时（秒）
    pub timeout: Option<u64>,
}

impl Request {
    /// 创建新请求
    pub fn new(kind: RequestKind) -> Self {
        Request {
            id: uuid::Uuid::new_v4().as_u128() as u64,
            kind,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            timeout: None,
        }
    }

    /// 设置超时
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 估算大小
    pub fn estimated_size(&self) -> usize {
        match &self.kind {
            RequestKind::Ping => 32,
            RequestKind::GetNode { .. } => 64,
            RequestKind::CreateNode { name, .. } => 128 + name.len(),
            RequestKind::UpdateNode { .. } => 64,
            RequestKind::DeleteNode { .. } => 64,
            RequestKind::QueryGraph { query } => 64 + query.len(),
            RequestKind::GetChildren { .. } => 64,
            RequestKind::FileOperation { path, .. } => 256 + path.as_os_str().len(),
            RequestKind::Batch { requests } => {
                requests.iter().map(|r| r.estimated_size()).sum()
            }
            RequestKind::Stream { .. } => 32,
        }
    }
}

/// 便捷方法：创建 Ping 请求
impl Request {
    pub fn ping() -> Self {
        Self::new(RequestKind::Ping)
    }

    pub fn get_node(id: NodeId) -> Self {
        Self::new(RequestKind::GetNode { id })
    }

    pub fn create_node(node_type: NodeType, name: String, parent_id: Option<NodeId>) -> Self {
        Self::new(RequestKind::CreateNode {
            node_type,
            name,
            parent_id,
        })
    }

    pub fn delete_node(id: NodeId) -> Self {
        Self::new(RequestKind::DeleteNode { id })
    }

    pub fn query_graph(query: String) -> Self {
        Self::new(RequestKind::QueryGraph { query })
    }

    pub fn get_children(id: NodeId) -> Self {
        Self::new(RequestKind::GetChildren { id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let req = Request::ping();
        assert!(matches!(req.kind, RequestKind::Ping));

        let id = NodeId::new_v4();
        let req = Request::get_node(id);
        assert!(matches!(req.kind, RequestKind::GetNode { .. }));
    }

    #[test]
    fn test_request_with_timeout() {
        let req = Request::ping().with_timeout(30);
        assert_eq!(req.timeout, Some(30));
    }

    #[test]
    fn test_file_operation() {
        let op = FileOperation::Open { flags: 0o644 };
        let req = Request::new(RequestKind::FileOperation {
            path: PathBuf::from("/test/file.txt"),
            operation: op,
        });

        match req.kind {
            RequestKind::FileOperation { path, operation } => {
                assert_eq!(path, PathBuf::from("/test/file.txt"));
                assert!(matches!(operation, FileOperation::Open { .. }));
            }
            _ => panic!("Expected file operation"),
        }
    }

    #[test]
    fn test_batch_request() {
        let req1 = Request::ping();
        let req2 = Request::ping();

        let batch = Request::new(RequestKind::Batch {
            requests: vec![req1, req2],
        });

        assert!(batch.estimated_size() > 0);
    }
}
