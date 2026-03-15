// 目录抽象

use crate::error::VfsResult;
use crate::file::FileType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 目录项
///
/// 表示目录中的一个条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    /// 名称
    pub name: String,

    /// 节点 ID
    pub ino: u64,

    /// 文件类型
    pub file_type: FileType,
}

impl DirEntry {
    /// 创建新的目录项
    pub fn new(name: impl Into<String>, ino: u64, file_type: FileType) -> Self {
        DirEntry {
            name: name.into(),
            ino,
            file_type,
        }
    }

    /// 是否为目录
    pub fn is_directory(&self) -> bool {
        self.file_type.is_directory()
    }

    /// 是否为普通文件
    pub fn is_file(&self) -> bool {
        self.file_type.is_regular()
    }

    /// 是否为符号链接
    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }
}

/// 目录
///
/// 表示一个目录的完整内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    /// 条目映射（名称 -> 目录项）
    entries: HashMap<String, DirEntry>,

    /// 目录的节点 ID
    ino: u64,
}

impl Directory {
    /// 创建新的空目录
    pub fn new(ino: u64) -> Self {
        Directory {
            entries: HashMap::new(),
            ino,
        }
    }

    /// 添加条目
    pub fn insert(&mut self, entry: DirEntry) -> VfsResult<()> {
        let name = entry.name.clone();
        if self.entries.contains_key(&name) {
            return Err(crate::error::VfsError::FileExists(name));
        }
        self.entries.insert(name, entry);
        Ok(())
    }

    /// 移除条目
    pub fn remove(&mut self, name: &str) -> Option<DirEntry> {
        self.entries.remove(name)
    }

    /// 获取条目
    pub fn get(&self, name: &str) -> Option<&DirEntry> {
        self.entries.get(name)
    }

    /// 检查条目是否存在
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// 列出所有条目
    pub fn entries(&self) -> Vec<DirEntry> {
        self.entries.values().cloned().collect()
    }

    /// 条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 获取节点 ID
    pub fn ino(&self) -> u64 {
        self.ino
    }

    /// 清空目录
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 迭代条目
    pub fn iter(&self) -> impl Iterator<Item = &DirEntry> {
        self.entries.values()
    }
}

/// 目录构建器
pub struct DirectoryBuilder {
    ino: u64,
    entries: Vec<DirEntry>,
}

impl DirectoryBuilder {
    /// 创建新的构建器
    pub fn new(ino: u64) -> Self {
        DirectoryBuilder {
            ino,
            entries: Vec::new(),
        }
    }

    /// 添加条目
    pub fn add_entry(mut self, entry: DirEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// 添加当前目录引用
    pub fn add_current(mut self) -> Self {
        self.entries
            .push(DirEntry::new(".", self.ino, FileType::Directory));
        self
    }

    /// 添加父目录引用
    pub fn add_parent(mut self, parent_ino: u64) -> Self {
        self.entries
            .push(DirEntry::new("..", parent_ino, FileType::Directory));
        self
    }

    /// 构建目录
    pub fn build(self) -> Directory {
        let mut dir = Directory::new(self.ino);
        for entry in self.entries {
            let _ = dir.insert(entry);
        }
        dir
    }
}

/// 目录迭代器
///
/// 用于遍历目录内容
#[derive(Debug)]
pub struct DirectoryIterator {
    entries: Vec<DirEntry>,
    position: usize,
}

impl DirectoryIterator {
    /// 创建新的迭代器
    pub fn new(entries: Vec<DirEntry>) -> Self {
        DirectoryIterator {
            entries,
            position: 0,
        }
    }

    /// 获取下一个条目
    pub fn next(&mut self) -> Option<DirEntry> {
        if self.position < self.entries.len() {
            let entry = self.entries[self.position].clone();
            self.position += 1;
            Some(entry)
        } else {
            None
        }
    }

    /// 重置迭代器
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// 是否还有更多条目
    pub fn has_more(&self) -> bool {
        self.position < self.entries.len()
    }

    /// 剩余条目数
    pub fn remaining(&self) -> usize {
        self.entries.len().saturating_sub(self.position)
    }
}

/// 目录操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryOperation {
    /// 创建成功
    Created(u64),

    /// 删除成功
    Deleted,

    /// 移动成功
    Moved,

    /// 已存在
    Exists,
}

/// 路径组件
///
/// 用于路径解析
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathComponent {
    /// 名称
    pub name: String,

    /// 是否为根目录
    pub is_root: bool,

    /// 是否为当前目录
    pub is_current: bool,

    /// 是否为父目录
    pub is_parent: bool,
}

impl PathComponent {
    /// 创建新的路径组件
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        PathComponent {
            is_root: name_str == "/",
            is_current: name_str == ".",
            is_parent: name_str == "..",
            name: name_str,
        }
    }

    /// 创建根目录组件
    pub fn root() -> Self {
        PathComponent {
            name: String::from("/"),
            is_root: true,
            is_current: false,
            is_parent: false,
        }
    }

    /// 创建当前目录组件
    pub fn current() -> Self {
        PathComponent {
            name: String::from("."),
            is_root: false,
            is_current: true,
            is_parent: false,
        }
    }

    /// 创建父目录组件
    pub fn parent() -> Self {
        PathComponent {
            name: String::from(".."),
            is_root: false,
            is_current: false,
            is_parent: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_entry() {
        let entry = DirEntry::new("test.txt", 123, FileType::Regular);
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.ino, 123);
        assert!(entry.is_file());
        assert!(!entry.is_directory());
    }

    #[test]
    fn test_directory() {
        let mut dir = Directory::new(1);

        assert!(dir.is_empty());
        assert_eq!(dir.len(), 0);

        let entry1 = DirEntry::new("file1.txt", 2, FileType::Regular);
        let entry2 = DirEntry::new("file2.txt", 3, FileType::Regular);

        dir.insert(entry1.clone()).unwrap();
        dir.insert(entry2.clone()).unwrap();

        assert_eq!(dir.len(), 2);
        assert!(dir.contains("file1.txt"));
        assert!(dir.contains("file2.txt"));

        let retrieved = dir.get("file1.txt");
        assert_eq!(retrieved, Some(&entry1));

        let removed = dir.remove("file1.txt");
        assert_eq!(removed, Some(entry1));
        assert_eq!(dir.len(), 1);
    }

    #[test]
    fn test_directory_duplicate() {
        let mut dir = Directory::new(1);

        let entry1 = DirEntry::new("test.txt", 2, FileType::Regular);
        let entry2 = DirEntry::new("test.txt", 3, FileType::Regular);

        dir.insert(entry1).unwrap();
        let result = dir.insert(entry2);
        assert!(result.is_err());
    }

    #[test]
    fn test_directory_builder() {
        let dir = DirectoryBuilder::new(1)
            .add_current()
            .add_parent(0)
            .add_entry(DirEntry::new("file.txt", 2, FileType::Regular))
            .build();

        assert_eq!(dir.len(), 3);
        assert!(dir.contains("."));
        assert!(dir.contains(".."));
        assert!(dir.contains("file.txt"));
    }

    #[test]
    fn test_directory_iterator() {
        let entries = vec![
            DirEntry::new("file1.txt", 1, FileType::Regular),
            DirEntry::new("file2.txt", 2, FileType::Regular),
            DirEntry::new("file3.txt", 3, FileType::Regular),
        ];

        let mut iter = DirectoryIterator::new(entries);

        assert_eq!(iter.remaining(), 3);
        assert!(iter.has_more());

        let entry1 = iter.next().unwrap();
        assert_eq!(entry1.name, "file1.txt");
        assert_eq!(iter.remaining(), 2);

        iter.next();
        iter.next();

        assert!(!iter.has_more());
        assert_eq!(iter.next(), None);

        iter.reset();
        assert!(iter.has_more());
    }

    #[test]
    fn test_path_component() {
        let root = PathComponent::root();
        assert!(root.is_root);
        assert!(!root.is_current);

        let current = PathComponent::current();
        assert!(current.is_current);

        let parent = PathComponent::parent();
        assert!(parent.is_parent);

        let normal = PathComponent::new("test.txt");
        assert!(!normal.is_root);
        assert!(!normal.is_current);
        assert!(!normal.is_parent);
        assert_eq!(normal.name, "test.txt");
    }
}
