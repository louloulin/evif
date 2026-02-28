// 路径解析器 - 将文件系统路径映射到图节点

use crate::error::{VfsError, VfsResult};
use evif_graph::{Graph, NodeId, NodeType, Attribute};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use dashmap::DashMap;

/// 路径解析器
///
/// 负责将 POSIX 风格的文件路径解析为图节点 ID
/// 支持绝对路径、相对路径、符号链接解析
pub struct PathResolver {
    graph: Arc<Graph>,
    root_id: NodeId,
    cache: DashMap<PathBuf, NodeId>,
    max_symlink_depth: usize,
}

impl PathResolver {
    /// 创建新的路径解析器
    pub fn new(graph: Arc<Graph>, root_id: NodeId) -> Self {
        PathResolver {
            graph,
            root_id,
            cache: DashMap::new(),
            max_symlink_depth: 40,
        }
    }

    /// 解析路径到节点 ID
    pub fn resolve(&self, path: &Path) -> VfsResult<NodeId> {
        let normalized = self.normalize_path(path)?;

        // 检查缓存
        if let Some(cached) = self.cache.get(&normalized) {
            return Ok(*cached);
        }

        // 解析路径
        let node_id = self.resolve_path(&normalized, 0)?;

        // 缓存结果
        self.cache.insert(normalized, node_id);

        Ok(node_id)
    }

    /// 规范化路径
    fn normalize_path(&self, path: &Path) -> VfsResult<PathBuf> {
        let path_str = path.to_str()
            .ok_or_else(|| VfsError::InvalidPath("路径包含无效字符".to_string()))?;

        if path_str.len() > 4096 {
            return Err(VfsError::PathTooLong);
        }

        let mut components = Vec::new();
        let mut is_absolute = false;

        // 检查是否为绝对路径
        if path_str.starts_with('/') {
            is_absolute = true;
        }

        for component in path.components() {
            use std::path::Component;
            match component {
                Component::Prefix(_) => {
                    return Err(VfsError::InvalidPath("不支持 Windows 路径前缀".to_string()));
                }
                Component::RootDir => {
                    // 根目录，跳过
                }
                Component::CurDir => {
                    // 当前目录，跳过
                }
                Component::ParentDir => {
                    // 父目录
                    if !components.is_empty() {
                        components.pop();
                    }
                }
                Component::Normal(name) => {
                    let name_str = name.to_str()
                        .ok_or_else(|| VfsError::InvalidPath("路径包含无效字符".to_string()))?;

                    if name_str.len() > 255 {
                        return Err(VfsError::NameTooLong);
                    }

                    // 检查是否包含无效字符
                    if name_str.contains('\0') {
                        return Err(VfsError::InvalidPath("路径包含空字符".to_string()));
                    }

                    components.push(name_str.to_string());
                }
            }
        }

        let normalized = if is_absolute {
            PathBuf::from("/").join(components.join("/"))
        } else {
            PathBuf::from(components.join("/"))
        };

        Ok(normalized)
    }

    /// 递归解析路径（同步版本）
    fn resolve_path(&self, path: &Path, depth: usize) -> VfsResult<NodeId> {
        if depth > self.max_symlink_depth {
            return Err(VfsError::SymbolicLinkLoop(path.display().to_string()));
        }

        let path_str = path.to_str()
            .ok_or_else(|| VfsError::InvalidPath("路径包含无效字符".to_string()))?;

        // 空路径或根路径
        if path_str == "/" || path_str.is_empty() {
            return Ok(self.root_id);
        }

        let mut current_id = self.root_id;

        for component in path.components() {
            use std::path::Component;
            if let Component::Normal(name) = component {
                let name_str = name.to_str()
                    .ok_or_else(|| VfsError::InvalidPath("路径包含无效字符".to_string()))?;

                // 查找子节点
                current_id = self.find_child(current_id, name_str)?;

                // 检查是否为符号链接
                if let Ok(node) = self.graph.get_node(&current_id) {
                    if node.node_type == NodeType::Symlink {
                        // 解析符号链接目标
                        let target = self.get_symlink_target(&node)?;
                        let target_path = path.parent()
                            .unwrap_or(Path::new("/"))
                            .join(&target);

                        // 递归解析
                        current_id = self.resolve_path(&target_path, depth + 1)?;
                    }
                }
            }
        }

        Ok(current_id)
    }

    /// 查找子节点（同步版本）
    fn find_child(&self, parent_id: NodeId, name: &str) -> VfsResult<NodeId> {
        // 获取父节点的所有出边 - 使用邻接表
        use evif_graph::Graph;

        // 图的内部结构不直接暴露，我们需要通过节点关系来查找
        // 这是一个简化实现，实际中可能需要在 Graph 中添加辅助方法

        // 暂时返回错误，表示需要实现图遍历
        Err(VfsError::PathNotFound(format!("未找到子节点: {}", name)))
    }

    /// 获取符号链接目标
    fn get_symlink_target(&self, node: &evif_graph::Node) -> VfsResult<String> {
        if let Some(attr) = node.get_attr("symlink_target") {
            if let Attribute::String(target) = attr {
                return Ok(target.clone());
            }
        }
        Err(VfsError::InvalidOperation("符号链接目标未设置".to_string()))
    }

    /// 创建路径到节点的映射
    pub fn create_path(&self, path: &Path, node_id: NodeId) -> VfsResult<()> {
        let normalized = self.normalize_path(path)?;
        self.cache.insert(normalized, node_id);
        Ok(())
    }

    /// 移除路径缓存
    pub fn invalidate(&self, path: &Path) {
        let normalized = match self.normalize_path(path) {
            Ok(p) => p,
            Err(_) => return,
        };

        // 移除精确匹配的缓存
        self.cache.remove(&normalized);

        // 移除所有前缀匹配的缓存
        self.cache.retain(|k, _| !k.starts_with(&normalized));
    }

    /// 获取父路径
    pub fn parent_path(&self, path: &Path) -> VfsResult<Option<PathBuf>> {
        let normalized = self.normalize_path(path)?;
        Ok(normalized.parent().map(|p| p.to_path_buf()))
    }

    /// 获取基础名称
    pub fn basename(&self, path: &Path) -> VfsResult<String> {
        let normalized = self.normalize_path(path)?;
        normalized
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| VfsError::InvalidPath("无法获取文件名".to_string()))
    }

    /// 清空缓存
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        let graph = Arc::new(Graph::new());
        let root_id = NodeId::new_v4();
        let resolver = PathResolver::new(graph, root_id);

        // 绝对路径
        let path = resolver.normalize_path(Path::new("/foo/bar")).unwrap();
        assert_eq!(path, PathBuf::from("/foo/bar"));

        // 相对路径
        let path = resolver.normalize_path(Path::new("foo/bar")).unwrap();
        assert_eq!(path, PathBuf::from("foo/bar"));

        // 父目录
        let path = resolver.normalize_path(Path::new("/foo/../bar")).unwrap();
        assert_eq!(path, PathBuf::from("/bar"));

        // 当前目录
        let path = resolver.normalize_path(Path::new("/foo/./bar")).unwrap();
        assert_eq!(path, PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_basename() {
        let graph = Arc::new(Graph::new());
        let root_id = NodeId::new_v4();
        let resolver = PathResolver::new(graph, root_id);

        assert_eq!(resolver.basename(Path::new("/foo/bar.txt")).unwrap(), "bar.txt");
        assert_eq!(resolver.basename(Path::new("/foo/bar/")).unwrap(), "bar");
    }

    #[test]
    fn test_path_too_long() {
        let graph = Arc::new(Graph::new());
        let root_id = NodeId::new_v4();
        let resolver = PathResolver::new(graph, root_id);

        let long_path = "/".repeat(5000);
        let result = resolver.normalize_path(Path::new(&long_path));
        assert!(matches!(result, Err(VfsError::PathTooLong)));
    }

    #[test]
    fn test_parent_path() {
        let graph = Arc::new(Graph::new());
        let root_id = NodeId::new_v4();
        let resolver = PathResolver::new(graph, root_id);

        let parent = resolver.parent_path(Path::new("/foo/bar/baz")).unwrap();
        assert_eq!(parent, Some(PathBuf::from("/foo/bar")));
    }
}
