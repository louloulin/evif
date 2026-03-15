// 虚拟节点 - VNode 抽象

use crate::error::VfsResult;
use evif_graph::{Node, NodeId};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 虚拟节点 (VNode)
///
/// VNode 是文件系统中活跃的节点表示，包含：
/// - 对应的图节点
/// - 节点数据缓存
/// - 引用计数
/// - 状态标志
#[derive(Debug, Clone)]
pub struct VNode {
    /// 节点 ID
    pub id: NodeId,

    /// 节点数据
    data: Arc<RwLock<VNodeData>>,

    /// 引用计数
    ref_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl VNode {
    /// 创建新的虚拟节点
    pub fn new(id: NodeId, data: VNodeData) -> Self {
        VNode {
            id,
            data: Arc::new(RwLock::new(data)),
            ref_count: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        }
    }

    /// 从图节点创建虚拟节点
    pub fn from_node(node: Node) -> Self {
        // 获取大小，优先从 ContentHandle，否则为 0
        let size = node.content.as_ref().map(|c| c.size).unwrap_or(0);

        // 转换时间戳
        let atime = node.metadata.accessed_at.timestamp() as u64;
        let mtime = node.metadata.modified_at.timestamp() as u64;
        let ctime = node.metadata.created_at.timestamp() as u64;

        let data = VNodeData {
            node_id: node.id,
            node_type: node.node_type,
            name: node.name,
            size,
            mode: node.metadata.permissions,
            uid: 0,
            gid: 0,
            atime,
            mtime,
            ctime,
            content: node.content,
        };

        VNode::new(node.id, data)
    }

    /// 获取节点 ID
    pub fn node_id(&self) -> NodeId {
        self.id
    }

    /// 获取节点数据
    pub async fn data(&self) -> VNodeData {
        self.data.read().await.clone()
    }

    /// 更新节点数据
    pub async fn update_data<F>(&self, f: F) -> VfsResult<()>
    where
        F: FnOnce(&mut VNodeData),
    {
        let mut data = self.data.write().await;
        f(&mut data);
        Ok(())
    }

    /// 增加引用计数
    pub fn inc_ref(&self) {
        self.ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 减少引用计数
    pub fn dec_ref(&self) -> usize {
        self.ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_sub(1)
    }

    /// 获取引用计数
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 检查是否被引用
    pub fn is_referenced(&self) -> bool {
        self.ref_count() > 0
    }

    /// 转换为图节点
    pub async fn to_graph_node(&self) -> Node {
        let data = self.data.read().await;
        let mut node = Node::new(data.node_type.clone(), &data.name);
        node.id = data.node_id;

        node.metadata.permissions = data.mode;

        // 转换时间戳回 DateTime
        use chrono::{DateTime, TimeZone, Utc};
        node.metadata.accessed_at =
            DateTime::<Utc>::from_timestamp(data.atime as i64, 0).unwrap_or_else(|| Utc::now());
        node.metadata.modified_at =
            DateTime::<Utc>::from_timestamp(data.mtime as i64, 0).unwrap_or_else(|| Utc::now());
        node.metadata.created_at =
            DateTime::<Utc>::from_timestamp(data.ctime as i64, 0).unwrap_or_else(|| Utc::now());

        if let Some(content) = &data.content {
            node.content = Some(content.clone());
        }

        node
    }
}

/// 虚拟节点数据
#[derive(Debug, Clone)]
pub struct VNodeData {
    /// 节点 ID
    pub node_id: NodeId,

    /// 节点类型
    pub node_type: evif_graph::NodeType,

    /// 节点名称
    pub name: String,

    /// 文件大小
    pub size: u64,

    /// 权限模式
    pub mode: u32,

    /// 用户 ID
    pub uid: u32,

    /// 组 ID
    pub gid: u32,

    /// 访问时间
    pub atime: u64,

    /// 修改时间
    pub mtime: u64,

    /// 创建时间
    pub ctime: u64,

    /// 内容句柄
    pub content: Option<evif_graph::ContentHandle>,
}

impl Default for VNodeData {
    fn default() -> Self {
        VNodeData {
            node_id: NodeId::new_v4(),
            node_type: evif_graph::NodeType::File,
            name: String::new(),
            size: 0,
            mode: 0o644,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            content: None,
        }
    }
}

/// VNode 构建器
pub struct VNodeBuilder {
    id: Option<NodeId>,
    node_type: evif_graph::NodeType,
    name: String,
    size: u64,
    mode: u32,
}

impl Default for VNodeBuilder {
    fn default() -> Self {
        VNodeBuilder {
            id: None,
            node_type: evif_graph::NodeType::File,
            name: String::new(),
            size: 0,
            mode: 0o644,
        }
    }
}

impl VNodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_type(mut self, node_type: evif_graph::NodeType) -> Self {
        self.node_type = node_type;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_mode(mut self, mode: u32) -> Self {
        self.mode = mode;
        self
    }

    pub fn build(self) -> VNode {
        let data = VNodeData {
            node_id: self.id.unwrap_or_else(NodeId::new_v4),
            node_type: self.node_type,
            name: self.name,
            size: self.size,
            mode: self.mode,
            ..Default::default()
        };

        VNode::new(data.node_id, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::NodeType;

    #[test]
    fn test_vnode_creation() {
        let vnode = VNodeBuilder::new()
            .with_type(NodeType::File)
            .with_name("test.txt")
            .with_size(1024)
            .with_mode(0o644)
            .build();

        assert_eq!(vnode.ref_count(), 1);
        assert!(vnode.is_referenced());
    }

    #[test]
    fn test_vnode_ref_count() {
        let vnode = VNodeBuilder::new().with_name("test.txt").build();

        vnode.inc_ref();
        assert_eq!(vnode.ref_count(), 2);

        vnode.dec_ref();
        assert_eq!(vnode.ref_count(), 1);
    }

    #[tokio::test]
    async fn test_vnode_data() {
        let vnode = VNodeBuilder::new()
            .with_name("test.txt")
            .with_size(2048)
            .build();

        let data = vnode.data().await;
        assert_eq!(data.name, "test.txt");
        assert_eq!(data.size, 2048);
    }

    #[tokio::test]
    async fn test_vnode_update() {
        let vnode = VNodeBuilder::new().with_name("test.txt").build();

        vnode
            .update_data(|data| {
                data.size = 4096;
            })
            .await
            .unwrap();

        let data = vnode.data().await;
        assert_eq!(data.size, 4096);
    }
}
