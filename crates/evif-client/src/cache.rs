// 客户端缓存

use evif_graph::{Node, NodeId};
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 客户端缓存
pub struct ClientCache {
    cache: Arc<Mutex<LruCache<NodeId, Node>>>,
}

impl ClientCache {
    /// 创建新缓存
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
        }
    }

    /// 获取节点
    pub async fn get(&self, id: &NodeId) -> Option<Node> {
        let mut cache = self.cache.lock().await;
        cache.get(id).cloned()
    }

    /// 放入节点
    pub async fn put(&self, id: NodeId, node: Node) {
        let mut cache = self.cache.lock().await;
        cache.put(id, node);
    }

    /// 移除节点
    pub async fn remove(&self, id: &NodeId) {
        let mut cache = self.cache.lock().await;
        cache.pop(id);
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    /// 获取缓存大小
    pub async fn len(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::NodeType;

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = ClientCache::new(10);
        let id = NodeId::new_v4();
        let node = Node::new(NodeType::File, "test.txt");

        cache.put(id, node.clone()).await;
        let retrieved = cache.get(&id).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let cache = ClientCache::new(10);
        let id = NodeId::new_v4();
        let node = Node::new(NodeType::File, "test.txt");

        cache.put(id, node).await;
        cache.remove(&id).await;

        let retrieved = cache.get(&id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = ClientCache::new(10);
        let id = NodeId::new_v4();
        let node = Node::new(NodeType::File, "test.txt");

        cache.put(id, node).await;
        assert_eq!(cache.len().await, 1);

        cache.clear().await;
        assert_eq!(cache.len().await, 0);
    }
}
