// EVIF Plugin Trait - 核心插件接口
//
// 对标 AGFS FileSystem 接口
// 所有插件实现此 trait 即可挂载到 EVIF 系统
// Phase 8: 增加 Validate/GetReadme/GetConfigParams（对标 AGFS ServicePlugin）

use crate::error::{EvifError, EvifResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 插件配置参数元数据（Phase 8.1，对标 AGFS ConfigParameter）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigParam {
    pub name: String,
    pub param_type: String, // "string" | "int" | "bool" | "object" 等
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// 文件信息（对标 AGFS FileInfo）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub mode: u32,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub is_dir: bool,
}

// 打开标志（对标 AGFS OpenFlag）
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ_ONLY = 1 << 0;    // 只读
        const WRITE_ONLY = 1 << 1;   // 只写
        const READ_WRITE = 1 << 2;   // 读写
        const CREATE = 1 << 3;       // 创建文件
        const EXCLUSIVE = 1 << 4;    // 排他创建
        const TRUNCATE = 1 << 5;     // 截断文件
        const APPEND = 1 << 6;       // 追加模式
        const NONBLOCK = 1 << 7;     // 非阻塞
    }
}

impl Default for OpenFlags {
    fn default() -> Self {
        OpenFlags::READ_ONLY
    }
}

/// 文件句柄接口（对标 AGFS FileHandle）
///
/// 提供有状态的文件操作,用于大文件分块传输和会话管理
#[async_trait]
pub trait FileHandle: Send + Sync {
    /// 返回句柄的唯一标识符
    fn id(&self) -> i64;

    /// 返回句柄关联的文件路径
    fn path(&self) -> &str;

    /// 从当前位置读取最多 buf.len() 字节
    async fn read(&mut self, buf: &mut [u8]) -> EvifResult<usize>;

    /// 从指定偏移量读取 (pread)
    async fn read_at(&self, buf: &mut [u8], offset: u64) -> EvifResult<usize>;

    /// 在当前位置写入数据
    async fn write(&mut self, data: &[u8]) -> EvifResult<usize>;

    /// 在指定偏移量写入 (pwrite)
    async fn write_at(&self, data: &[u8], offset: u64) -> EvifResult<usize>;

    /// 移动读写位置
    ///
    /// # 参数
    /// - `offset`: 偏移量
    /// - `whence`: 0=SEEK_SET(从开始), 1=SEEK_CUR(从当前), 2=SEEK_END(从结束)
    ///
    /// # 返回
    /// 新的读写位置
    async fn seek(&mut self, offset: i64, whence: u8) -> EvifResult<i64>;

    /// 同步文件数据到存储
    async fn sync(&self) -> EvifResult<()>;

    /// 关闭句柄并释放资源
    async fn close(&mut self) -> EvifResult<()>;

    /// 获取文件信息
    async fn stat(&self) -> EvifResult<FileInfo>;

    /// 返回打开此句柄时使用的标志
    fn flags(&self) -> OpenFlags;
}

/// HandleFS 接口 - 支持有状态文件句柄的文件系统
///
/// 这是可选的扩展接口,不支持的文件系统可以不实现
#[async_trait]
pub trait HandleFS: EvifPlugin {
    /// 打开文件并返回用于有状态操作的句柄
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `flags`: 打开标志位
    /// - `mode`: 文件权限模式（创建新文件时使用）
    ///
    /// # 返回
    /// 文件句柄
    async fn open_handle(
        &self,
        path: &str,
        flags: OpenFlags,
        mode: u32,
    ) -> EvifResult<Box<dyn FileHandle>>;

    /// 根据 ID 获取已存在的句柄
    ///
    /// 如果句柄不存在或已过期,返回 ErrNotFound
    async fn get_handle(&self, id: i64) -> EvifResult<Box<dyn FileHandle>>;

    /// 根据 ID 关闭句柄
    ///
    /// 等同于调用 handle.close(),但适用于只有 ID 可用的情况
    async fn close_handle(&self, id: i64) -> EvifResult<()>;
}

// 写入标志（对标 AGFS WriteFlag）
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WriteFlags: u32 {
        const NONE = 0;
        const APPEND = 1 << 0;      // 追加写入
        const CREATE = 1 << 1;      // 创建文件
        const EXCLUSIVE = 1 << 2;   // 排他创建
        const TRUNCATE = 1 << 3;    // 截断文件
        const SYNC = 1 << 4;        // 同步写入
    }
}

impl Default for WriteFlags {
    fn default() -> Self {
        WriteFlags::NONE
    }
}

/// EVIF 插件接口（对标 AGFS FileSystem）
///
/// 所有插件必须实现此 trait，提供标准的 POSIX 文件操作
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;

    /// 创建文件
    async fn create(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// 创建目录
    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>;

    /// 读取文件
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `offset`: 读取偏移量（0 表示从头开始）
    /// - `size`: 读取大小（0 表示读取全部）
    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>>;

    /// 写入文件
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `data`: 写入数据
    /// - `offset`: 写入偏移量（-1 表示忽略）
    /// - `flags`: 写入标志
    ///
    /// # 返回
    /// 实际写入的字节数
    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64>;

    /// 读取目录内容
    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>;

    /// 获取文件信息
    async fn stat(&self, path: &str) -> EvifResult<FileInfo>;

    /// 删除文件或空目录
    async fn remove(&self, path: &str) -> EvifResult<()>;

    /// 重命名/移动文件
    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()>;

    /// 递归删除目录及其所有内容
    ///
    /// # 行为
    /// - 删除指定路径下的所有文件和子目录
    /// - 如果路径是文件,等同于 remove()
    /// - 如果路径是目录,递归删除所有子项后删除目录本身
    ///
    /// # AGFS 对标
    /// ```go
    /// func (fs *FileSystem) RemoveAll(path string) error
    /// ```
    async fn remove_all(&self, path: &str) -> EvifResult<()>;

    /// 创建符号链接
    ///
    /// # 参数
    /// - `target_path`: 链接目标路径
    /// - `link_path`: 符号链接路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (fs Symlinker) Symlink(targetPath, linkPath string) error
    /// ```
    async fn symlink(&self, _target_path: &str, _link_path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    /// 读取符号链接目标
    ///
    /// # 参数
    /// - `link_path`: 符号链接路径
    ///
    /// # 返回
    /// 链接目标的路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (fs Symlinker) Readlink(linkPath string) (string, error)
    /// ```
    async fn readlink(&self, _link_path: &str) -> EvifResult<String> {
        Err(EvifError::NotSupportedGeneric)
    }

    /// 修改文件权限
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `mode`: 新的权限模式（八进制表示，如 0o644）
    ///
    /// # AGFS 对标
    /// ```go
    /// func (fs *FileSystem) Chmod(path string, mode uint32) error
    /// ```
    async fn chmod(&self, _path: &str, _mode: u32) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    /// 截断文件到指定大小
    ///
    /// # 参数
    /// - `path`: 文件路径
    /// - `size`: 新的文件大小（字节）
    ///
    /// # 行为
    /// - 如果文件大小 > size，文件将被截断（丢弃多余部分）
    /// - 如果文件大小 < size，文件将被扩展（使用空洞或零填充）
    ///
    /// # AGFS 对标
    /// ```go
    /// func (fs Truncator) Truncate(path string, size int64) error
    /// ```
    async fn truncate(&self, _path: &str, _size: u64) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    /// 校验插件配置（Phase 8.1，对标 AGFS Validate）
    ///
    /// mount 前调用；配置无效时返回 Err，阻止挂载
    async fn validate(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        Ok(())
    }

    /// 初始化插件
    ///
    /// 在 validate 成功后、正式挂载前调用。
    async fn initialize(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        Ok(())
    }

    /// 优雅关闭插件
    ///
    /// 在卸载或替换插件前调用。
    async fn shutdown(&self) -> EvifResult<()> {
        Ok(())
    }

    /// 热重载插件（Phase 16.1）
    ///
    /// 重新初始化插件，可能加载新的配置或更新内部状态。
    /// 默认实现调用 shutdown 然后 initialize。
    async fn reload(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        self.shutdown().await?;
        self.initialize(_config).await
    }

    /// 返回插件 README 文档（Phase 8.2，对标 AGFS GetReadme）
    fn get_readme(&self) -> String {
        String::new()
    }

    /// 返回插件配置参数元数据（Phase 8.1，对标 AGFS GetConfigParams）
    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![]
    }

    /// 转换为 Any（用于向下转型）
    fn as_any(&self) -> &dyn std::any::Any {
        // 默认实现，返回空类型
        &()
    }

    /// 转换为 HandleFS（如果支持）
    ///
    /// # 返回
    /// - Some(句柄) 如果插件支持 HandleFS
    /// - None 如果插件不支持 HandleFS
    fn as_handle_fs(&self) -> Option<&dyn crate::plugin::HandleFS> {
        None
    }

    /// 转换为 Streamer（如果支持流式操作）
    fn as_streamer(&self) -> Option<&dyn crate::streaming::Streamer> {
        None
    }
}

/// 统一执行插件生命周期前置步骤：先校验，再初始化。
pub async fn validate_and_initialize_plugin(
    plugin: &dyn EvifPlugin,
    config: Option<&serde_json::Value>,
) -> EvifResult<()> {
    plugin.validate(config).await?;
    plugin.initialize(config).await?;
    Ok(())
}
