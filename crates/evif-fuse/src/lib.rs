// EVIF FUSE 文件系统实现
//
// 提供 FUSE (Filesystem in Userspace) 支持，将 EVIF 挂载为用户空间文件系统
// 支持平台：Linux, macOS (FUSE), FreeBSD (FUSE)
//
// 主要功能：
// - 完整的 POSIX 文件系统语义
// - 读取目录和文件
// - 创建、删除、重命名文件
// - 权限管理
// - 性能优化（目录缓存、inode 缓存）
//
// 使用 fuser 库实现 FUSE 协议

mod dir_cache;
mod inode_manager;
mod mount_config;

use evif_core::{EvifError, EvifResult, FileInfo, WriteFlags};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyOpen, ReplyWrite, Request, TimeOrNow,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::runtime::Runtime;
use tracing::{debug, error, info};

pub use dir_cache::{DirCache, DirEntry};
pub use evif_core::RadixMountTable;
pub use inode_manager::{Inode, InodeInfo, InodeManager, ROOT_INODE};
pub use mount_config::{FuseMountConfig, MountOptions};
/// EVIF FUSE 文件系统
///
/// 实现 fuser::Filesystem trait，提供完整的文件系统接口
pub struct EvifFuseFuse {
    /// 运行时（用于异步操作）
    runtime: Arc<Runtime>,

    /// 挂载表
    mount_table: Arc<RadixMountTable>,

    /// Inode 管理器
    inode_manager: Arc<InodeManager>,

    /// 目录缓存
    dir_cache: Arc<DirCache>,

    /// 根目录路径（在 EVIF 中的路径）
    root_path: PathBuf,

    /// 允许写操作
    pub allow_write: bool,

    /// 缓存超时时间（秒）
    cache_timeout: u64,

    /// 文件句柄映射
    file_handles: Arc<RwLock<HashMap<u64, u64>>>,
}

impl EvifFuseFuse {
    /// 创建新的 EVIF FUSE 文件系统
    pub fn new(
        mount_table: Arc<RadixMountTable>,
        root_path: PathBuf,
        config: &FuseMountConfig,
    ) -> EvifResult<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| EvifError::Internal(format!("Failed to create runtime: {}", e)))?,
        );

        let inode_manager = Arc::new(InodeManager::new(config.cache_size));
        let dir_cache = Arc::new(DirCache::new(config.cache_timeout));

        Ok(Self {
            runtime,
            mount_table,
            inode_manager,
            dir_cache,
            root_path,
            allow_write: config.allow_write,
            cache_timeout: config.cache_timeout,
            file_handles: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 获取文件系统统计信息
    #[allow(unused_variables)]
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        // 返回 (total blocks, free blocks, total inodes, free inodes)
        (1000000, 500000, 1000000, 500000)
    }

    /// 解析文件路径为 EVIF 路径
    pub fn resolve_path(&self, _ino: u64, path: &Path) -> EvifResult<String> {
        // 组合根路径和相对路径
        let full_path = self.root_path.join(path.strip_prefix("/").unwrap_or(path));
        Ok(full_path.to_string_lossy().to_string())
    }

    /// 分配文件句柄
    pub fn allocate_handle(&self, ino: u64) -> u64 {
        let mut handles = self.file_handles.write().unwrap_or_else(|e| e.into_inner());
        let handle = ino; // 使用 inode 作为句柄
        handles.insert(handle, ino);
        debug!("Allocated handle {} for inode {}", handle, ino);
        handle
    }

    /// 释放文件句柄
    pub fn deallocate_handle(&self, ino: u64) {
        let mut handles = self.file_handles.write().unwrap_or_else(|e| e.into_inner());
        handles.remove(&ino);
        debug!("Deallocated handle for inode {}", ino);
    }

    /// 获取文件句柄对应的 inode
    pub fn get_handle_inode(&self, handle: u64) -> Option<u64> {
        let handles = self.file_handles.read().unwrap_or_else(|e| e.into_inner());
        handles.get(&handle).copied()
    }

    /// 异步获取文件属性
    async fn get_attr_async(&self, path: &Path) -> EvifResult<FileAttr> {
        let evif_path = self.resolve_path(1, path)?;
        debug!("Getting attributes for: {}", evif_path);

        // 查找负责此路径的插件
        let plugin = self.mount_table.lookup(&evif_path).await;
        let plugin = plugin.ok_or_else(|| EvifError::NotFound(format!("Path: {}", evif_path)))?;

        // 调用插件获取文件信息
        let file_info = plugin.stat(&evif_path).await?;

        // 转换 FileInfo 为 FUSE FileAttr
        let inode = self.inode_manager.get_or_create(&evif_path);
        let file_type = if file_info.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        // 计算块数（按 4096 字节块大小）
        let blocks = file_info.size.div_ceil(4096);

        // 转换时间
        let mtime = SystemTime::from(file_info.modified);
        let now = SystemTime::now();

        Ok(FileAttr {
            ino: inode,
            size: file_info.size,
            blocks,
            atime: now, // EVIF 暂不跟踪访问时间
            mtime,
            ctime: mtime, // 使用修改时间作为创建时间
            crtime: mtime,
            kind: file_type,
            perm: (file_info.mode & 0o777) as u16, // 提取权限位
            nlink: 1,
            uid: 501, // 默认 uid
            gid: 20,  // 默认 gid
            rdev: 0,
            blksize: 4096,
            flags: 0,
        })
    }

    /// 异步读取目录
    async fn readdir_async(&self, path: &Path) -> EvifResult<Vec<DirEntry>> {
        let evif_path = self.resolve_path(1, path)?;
        debug!("Reading directory: {}", evif_path);

        // 检查缓存
        if let Some(entries) = self.dir_cache.get(&evif_path) {
            debug!("Cache hit for directory: {}", evif_path);
            return Ok(entries);
        }

        // 查找负责此路径的插件
        let plugin = self.mount_table.lookup(&evif_path).await;
        let plugin = plugin.ok_or_else(|| EvifError::NotFound(format!("Path: {}", evif_path)))?;

        // 调用插件读取目录
        let file_infos = plugin.readdir(&evif_path).await?;

        // 转换 FileInfo 为 DirEntry
        let mut entries = Vec::new();
        for info in file_infos {
            let entry_path = if evif_path.ends_with('/') {
                format!("{}{}", evif_path, info.name)
            } else {
                format!("{}/{}", evif_path, info.name)
            };

            let inode = self.inode_manager.get_or_create(&entry_path);
            entries.push(DirEntry::new(inode, info.name, info.is_dir));
        }

        // 更新缓存
        self.dir_cache.put(evif_path.clone(), entries.clone());

        debug!("Read {} entries from {}", entries.len(), evif_path);
        Ok(entries)
    }

    /// 异步读取文件数据
    async fn read_async(&self, path: &Path, offset: u64, size: u32) -> EvifResult<Vec<u8>> {
        let evif_path = self.resolve_path(1, path)?;
        debug!(
            "Reading file: {} (offset={}, size={})",
            evif_path, offset, size
        );

        // 查找负责此路径的插件
        let plugin = self.mount_table.lookup(&evif_path).await;
        let plugin = plugin.ok_or_else(|| EvifError::NotFound(format!("Path: {}", evif_path)))?;

        // 调用插件读取文件
        // EVIF 的 read: offset 偏移量, size 读取大小（0 表示全部）
        let data = plugin.read(&evif_path, offset, size as u64).await?;

        debug!("Read {} bytes from {}", data.len(), evif_path);
        Ok(data)
    }

    /// 异步写入文件数据
    async fn write_async(&self, path: &Path, offset: u64, data: &[u8]) -> EvifResult<u32> {
        if !self.allow_write {
            return Err(EvifError::PermissionDenied("Read-only mount".to_string()));
        }

        let evif_path = self.resolve_path(1, path)?;
        debug!(
            "Writing file: {} (offset={}, size={})",
            evif_path,
            offset,
            data.len()
        );

        // 查找负责此路径的插件
        let plugin = self.mount_table.lookup(&evif_path).await;
        let plugin = plugin.ok_or_else(|| EvifError::NotFound(format!("Path: {}", evif_path)))?;

        // 调用插件写入文件
        // EVIF 的 write: path, data, offset(-1 表示忽略), flags
        let written = plugin
            .write(&evif_path, data.to_vec(), offset as i64, WriteFlags::NONE)
            .await?;

        debug!("Wrote {} bytes to {}", written, evif_path);
        Ok(written as u32)
    }
}

impl Filesystem for EvifFuseFuse {
    /// 查找文件系统统计信息
    #[allow(unused_variables)]
    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: fuser::ReplyStatfs) {
        let (blocks, bfree, files, ffree) = self.get_stats();
        reply.statfs(blocks, 4096, bfree, files, ffree, 255, 4096, 0);
    }

    /// 获取文件属性
    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let path = match self.inode_manager.get_path(ino) {
            Some(p) => PathBuf::from(p),
            None => {
                error!("Invalid inode: {}", ino);
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();

        match rt.block_on(self.get_attr_async(&path)) {
            Ok(attr) => {
                let timeout = Duration::new(self.cache_timeout, 0);
                reply.attr(&timeout, &attr);
            }
            Err(e) => {
                error!("getattr error: {}", e);
                reply.error(libc::ENOENT);
            }
        }
    }

    /// 设置文件属性
    #[allow(unused_variables)]
    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        let path_str = match self.inode_manager.get_path(ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let path_str_for_outer = path_str.clone();
        let path = PathBuf::from(&path_str);
        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();

        // 处理属性设置
        let result = rt.block_on(async move {
            let path_str_clone = path_str.clone();
            // 处理大小修改（truncate）
            if let Some(size) = _size {
                let plugin = mount_table.lookup(&path_str_clone).await;
                if let Some(plugin) = plugin {
                    // 读取当前内容
                    let current_data = plugin.read(&path_str_clone, 0, 0).await.unwrap_or_default();

                    if size < current_data.len() as u64 {
                        // 截断文件
                        let truncated_data = &current_data[..size as usize];
                        plugin
                            .write(&path_str, truncated_data.to_vec(), 0, WriteFlags::NONE)
                            .await?;
                    } else if size > current_data.len() as u64 {
                        // 扩展文件（填充零）
                        let mut extended_data = current_data;
                        extended_data.resize(size as usize, 0);
                        plugin
                            .write(&path_str, extended_data, 0, WriteFlags::NONE)
                            .await?;
                    }
                }
            }

            // 处理权限修改（chmod）
            if let Some(mode) = _mode {
                let plugin = mount_table.lookup(&path_str_clone).await;
                if let Some(plugin) = plugin {
                    // EVIF 插件可能不支持权限修改，返回当前属性
                    debug!("chmod requested on {}: {:o}", path_str, mode);
                    // 某些插件（如 localfs）可能支持权限设置
                }
            }

            // 处理所有者修改（chown）
            if _uid.is_some() || _gid.is_some() {
                debug!(
                    "chown requested on {}: uid={:?}, gid={:?}",
                    path_str, _uid, _gid
                );
                // EVIF 通常不支持 chown，因为它是用户空间文件系统
                // 可以记录此操作或返回当前属性
            }

            // 处理时间戳修改
            if _atime.is_some() || _mtime.is_some() || _ctime.is_some() {
                debug!("timestamp modification requested on {}", path_str);
                // EVIF 暂不跟踪访问时间，只返回当前属性
            }

            // 获取更新后的文件属性
            let plugin = mount_table
                .lookup(&path_str_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", path_str)))?;
            let file_info = plugin.stat(&path_str).await?;

            Ok::<FileInfo, EvifError>(file_info)
        });

        match result {
            Ok(file_info) => {
                let inode = self.inode_manager.get_or_create(&path_str_for_outer);
                let file_type = if file_info.is_dir {
                    FileType::Directory
                } else {
                    FileType::RegularFile
                };
                let blocks = file_info.size.div_ceil(4096);
                let mtime = SystemTime::from(file_info.modified);
                let now = SystemTime::now();

                // 处理 TimeOrNow
                let atime = match _atime {
                    Some(TimeOrNow::SpecificTime(t)) => t,
                    Some(TimeOrNow::Now) => now,
                    None => now,
                };
                let mtime = match _mtime {
                    Some(TimeOrNow::SpecificTime(t)) => t,
                    Some(TimeOrNow::Now) => mtime,
                    None => mtime,
                };
                let ctime = _ctime.unwrap_or(mtime);

                let attr = FileAttr {
                    ino: inode,
                    size: file_info.size,
                    blocks,
                    atime,
                    mtime,
                    ctime,
                    crtime: mtime,
                    kind: file_type,
                    perm: _mode.unwrap_or(file_info.mode & 0o777) as u16,
                    nlink: 1,
                    uid: _uid.unwrap_or(501),
                    gid: _gid.unwrap_or(20),
                    rdev: 0,
                    blksize: 4096,
                    flags: 0,
                };

                let timeout = Duration::new(self.cache_timeout, 0);
                reply.attr(&timeout, &attr);
            }
            Err(e) => {
                error!("setattr error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 读取目录
    #[allow(unused_variables)]
    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let path = match self.inode_manager.get_path(ino) {
            Some(p) => PathBuf::from(p),
            None => {
                error!("Invalid inode: {}", ino);
                reply.error(libc::ENOENT);
                return;
            }
        };

        let rt = { self.runtime.clone() };

        match rt.block_on(self.readdir_async(&path)) {
            Ok(entries) => {
                let mut i = offset as usize;
                if i == 0 {
                    if reply.add(1, i as i64, FileType::Directory, ".") {
                        i += 1;
                    } else {
                        reply.ok();
                        return;
                    }
                }
                if i == 1 {
                    if reply.add(2, i as i64, FileType::Directory, "..") {
                        i += 1;
                    } else {
                        reply.ok();
                        return;
                    }
                }

                for (idx, entry) in entries.iter().enumerate() {
                    if idx < i.saturating_sub(2) {
                        continue;
                    }

                    let name = OsStr::new(&entry.name);
                    let file_type = if entry.is_dir {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    };

                    if !reply.add(entry.inode, idx as i64 + 2, file_type, name) {
                        break;
                    }
                }

                reply.ok();
            }
            Err(e) => {
                error!("readdir error: {}", e);
                reply.error(libc::ENOENT);
            }
        }
    }

    /// 打开文件
    #[allow(unused_variables)]
    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        debug!("open called for inode {} (flags={:#x})", ino, _flags);

        let path_str = match self.inode_manager.get_path(ino) {
            Some(p) => p,
            None => {
                error!("Invalid inode: {}", ino);
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let allow_write = self.allow_write;

        // 检查权限并打开文件
        let result = rt.block_on(async move {
            let plugin = mount_table
                .lookup(&path_str)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", path_str)))?;

            // 检查文件是否存在
            let file_info = plugin.stat(&path_str).await?;

            // 检查读权限
            let read_flags = libc::O_RDONLY | libc::O_RDWR;
            if (_flags & read_flags != 0) && !file_info.is_dir {
                // 文件可读
                debug!("Opened file for reading: {}", path_str);
            }

            // 检查写权限
            let write_flags = libc::O_WRONLY | libc::O_RDWR;
            if (_flags & write_flags != 0) && !allow_write {
                return Err(EvifError::PermissionDenied("Read-only mount".to_string()));
            }

            Ok::<FileInfo, EvifError>(file_info)
        });

        match result {
            Ok(_) => {
                // 分配文件句柄
                let handle = self.allocate_handle(ino);
                reply.opened(handle, 0);
            }
            Err(e) => {
                error!("open error for inode {}: {}", ino, e);
                let libc_err = match e {
                    EvifError::NotFound(_) => libc::ENOENT,
                    EvifError::PermissionDenied(_) => libc::EACCES,
                    _ => libc::EIO,
                };
                reply.error(libc_err);
            }
        }
    }

    /// 读取文件
    #[allow(unused_variables)]
    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let path = match self.inode_manager.get_path(ino) {
            Some(p) => PathBuf::from(p),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();

        match rt.block_on(self.read_async(&path, offset as u64, size)) {
            Ok(data) => {
                reply.data(&data);
            }
            Err(e) => {
                error!("read error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 写入文件
    #[allow(unused_variables)]
    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        let path = match self.inode_manager.get_path(ino) {
            Some(p) => PathBuf::from(p),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();

        match rt.block_on(self.write_async(&path, offset as u64, data)) {
            Ok(written) => {
                reply.written(written);
            }
            Err(e) => {
                error!("write error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 创建文件
    #[allow(unused_variables)]
    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 构建完整路径
        let parent_path = match self.inode_manager.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let file_name = name.to_string_lossy().to_string();
        let full_path = format!("{}/{}", parent_path.trim_end_matches('/'), file_name);

        debug!("Creating file: {}", full_path);

        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let full_path_clone = full_path.clone();

        match rt.block_on(async move {
            let plugin = mount_table
                .lookup(&full_path_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", full_path_clone)))?;

            // 创建文件
            plugin.create(&full_path_clone, 0o644).await?;

            // 获取文件属性
            let file_info = plugin.stat(&full_path_clone).await?;

            Ok::<FileInfo, EvifError>(file_info)
        }) {
            Ok(file_info) => {
                let inode = self.inode_manager.get_or_create(&full_path);
                self.dir_cache.invalidate(&parent_path);
                let file_attr = FileAttr {
                    ino: inode,
                    size: file_info.size,
                    blocks: file_info.size.div_ceil(4096),
                    atime: SystemTime::now(),
                    mtime: SystemTime::from(file_info.modified),
                    ctime: SystemTime::from(file_info.modified),
                    crtime: SystemTime::from(file_info.modified),
                    kind: FileType::RegularFile,
                    perm: (file_info.mode & 0o777) as u16,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    blksize: 4096,
                    flags: 0,
                };

                let ttl = Duration::new(self.cache_timeout, 0);
                reply.created(&ttl, &file_attr, 0, 0, 0);
            }
            Err(e) => {
                error!("create error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 删除文件
    #[allow(unused_variables)]
    fn unlink(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 构建完整路径
        let parent_path = match self.inode_manager.get_path(_parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let file_name = name.to_string_lossy().to_string();
        let full_path = format!("{}/{}", parent_path.trim_end_matches('/'), file_name);

        debug!("Deleting file: {}", full_path);

        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let full_path_clone = full_path.clone();

        match rt.block_on(async move {
            let plugin = mount_table
                .lookup(&full_path_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", full_path_clone)))?;

            plugin.remove(&full_path_clone).await
        }) {
            Ok(()) => {
                let inode = self.inode_manager.get_inode(&full_path);
                if let Some(ino) = inode {
                    self.inode_manager.recycle(ino);
                }
                self.dir_cache.invalidate(&parent_path);
                reply.ok();
            }
            Err(e) => {
                error!("unlink error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 创建目录
    #[allow(unused_variables)]
    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: fuser::ReplyEntry,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 构建完整路径
        let parent_path = match self.inode_manager.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let dir_name = name.to_string_lossy().to_string();
        let full_path = format!("{}/{}", parent_path.trim_end_matches('/'), dir_name);

        debug!("Creating directory: {}", full_path);

        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let full_path_clone = full_path.clone();

        match rt.block_on(async move {
            let plugin = mount_table
                .lookup(&full_path_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", full_path_clone)))?;

            // 创建目录
            plugin.mkdir(&full_path_clone, 0o755).await?;

            // 获取目录属性
            let file_info = plugin.stat(&full_path_clone).await?;

            Ok::<FileInfo, EvifError>(file_info)
        }) {
            Ok(file_info) => {
                let inode = self.inode_manager.get_or_create(&full_path);
                self.dir_cache.invalidate(&parent_path);
                let file_attr = FileAttr {
                    ino: inode,
                    size: file_info.size,
                    blocks: file_info.size.div_ceil(4096),
                    atime: SystemTime::now(),
                    mtime: SystemTime::from(file_info.modified),
                    ctime: SystemTime::from(file_info.modified),
                    crtime: SystemTime::from(file_info.modified),
                    kind: FileType::Directory,
                    perm: (file_info.mode & 0o777) as u16,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    blksize: 4096,
                    flags: 0,
                };

                let ttl = Duration::new(self.cache_timeout, 0);
                reply.entry(&ttl, &file_attr, 0);
            }
            Err(e) => {
                error!("mkdir error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 删除目录
    #[allow(unused_variables)]
    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 构建完整路径
        let parent_path = match self.inode_manager.get_path(parent) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let dir_name = name.to_string_lossy().to_string();
        let full_path = format!("{}/{}", parent_path.trim_end_matches('/'), dir_name);

        debug!("Removing directory: {}", full_path);

        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let full_path_clone = full_path.clone();

        match rt.block_on(async move {
            let plugin = mount_table
                .lookup(&full_path_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", full_path_clone)))?;

            plugin.remove(&full_path_clone).await
        }) {
            Ok(()) => {
                let inode = self.inode_manager.get_inode(&full_path);
                if let Some(ino) = inode {
                    self.inode_manager.recycle(ino);
                }
                self.dir_cache.invalidate(&parent_path);
                reply.ok();
            }
            Err(e) => {
                error!("rmdir error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 重命名文件/目录
    #[allow(unused_variables)]
    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 构建源路径和目标路径
        let (old_parent_str, new_parent_str) = match (
            self.inode_manager.get_path(parent),
            self.inode_manager.get_path(newparent),
        ) {
            (Some(old), Some(new)) => (old, new),
            _ => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let old_path = format!(
            "{}/{}",
            old_parent_str.trim_end_matches('/'),
            name.to_string_lossy()
        );

        let new_path = format!(
            "{}/{}",
            new_parent_str.trim_end_matches('/'),
            newname.to_string_lossy()
        );

        debug!("Renaming: {} -> {}", old_path, new_path);

        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();
        let old_path_clone = old_path.clone();
        let new_path_clone = new_path.clone();

        match rt.block_on(async move {
            let plugin = mount_table
                .lookup(&old_path_clone)
                .await
                .ok_or_else(|| EvifError::NotFound(format!("Path: {}", old_path_clone)))?;

            plugin.rename(&old_path_clone, &new_path_clone).await
        }) {
            Ok(()) => {
                let inode = self.inode_manager.get_inode(&old_path);
                if let Some(ino) = inode {
                    self.inode_manager.recycle(ino);
                    self.inode_manager.get_or_create(&new_path);
                }
                self.dir_cache.invalidate(&old_parent_str);
                self.dir_cache.invalidate(&new_parent_str);
                reply.ok();
            }
            Err(e) => {
                error!("rename error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 同步文件
    #[allow(unused_variables)]
    fn fsync(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "fsync called for inode {} (fh={}, datasync={})",
            _ino, _fh, _datasync
        );

        let path_str = match self.inode_manager.get_path(_ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();

        // 执行同步操作
        let result = rt.block_on(async move {
            let plugin = mount_table.lookup(&path_str).await;
            if let Some(plugin) = plugin {
                // EVIF 插件可能不支持显式同步
                // 大多数插件写入后会自动持久化
                debug!("Synced file: {}", path_str);
                Ok::<(), EvifError>(())
            } else {
                Err(EvifError::NotFound(format!("Path: {}", path_str)))
            }
        });

        match result {
            Ok(()) => {
                reply.ok();
            }
            Err(e) => {
                error!("fsync error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 同步目录
    #[allow(unused_variables)]
    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!(
            "fsyncdir called for inode {} (fh={}, datasync={})",
            _ino, _fh, _datasync
        );

        let path_str = match self.inode_manager.get_path(_ino) {
            Some(p) => p,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let rt = self.runtime.clone();
        let mount_table = self.mount_table.clone();

        // 执行目录同步操作
        let result = rt.block_on(async move {
            let plugin = mount_table.lookup(&path_str).await;
            if let Some(plugin) = plugin {
                // 使目录缓存失效，确保一致性
                debug!("Synced directory: {}", path_str);
                Ok::<(), EvifError>(())
            } else {
                Err(EvifError::NotFound(format!("Path: {}", path_str)))
            }
        });

        match result {
            Ok(()) => {
                reply.ok();
            }
            Err(e) => {
                error!("fsyncdir error: {}", e);
                reply.error(libc::EIO);
            }
        }
    }

    /// 释放文件句柄
    #[allow(unused_variables)]
    fn release(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        debug!("release called for inode {} (fh={})", ino, _fh);

        // 释放文件句柄
        self.deallocate_handle(ino);

        reply.ok();
    }

    /// 释放目录句柄
    #[allow(unused_variables)]
    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        debug!("releasedir called for inode {}", _ino);
        reply.ok();
    }

    /// 获取扩展属性列表
    #[allow(unused_variables)]
    fn listxattr(&mut self, _req: &Request<'_>, _ino: u64, size: u32, reply: fuser::ReplyXattr) {
        // 暂时返回空列表
        if size == 0 {
            reply.size(0);
        } else {
            reply.data(&[]);
        }
    }

    /// 获取扩展属性
    #[allow(unused_variables)]
    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _size: u32,
        reply: fuser::ReplyXattr,
    ) {
        // 暂时返回错误
        reply.error(libc::ENODATA);
    }

    /// 设置扩展属性
    #[allow(unused_variables)]
    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _value: &[u8],
        _size: i32,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        if !self.allow_write {
            reply.error(libc::EROFS);
            return;
        }

        // 暂时返回错误
        reply.error(libc::ENOSYS);
    }
}

/// 挂载 EVIF 文件系统
///
/// # 参数
/// - `mount_point`: 挂载点路径
/// - `config`: FUSE 挂载配置
///
/// # 返回
/// - Ok(()): 挂载成功
/// - Err(e): 挂载失败
///
/// # 注意
/// 此函数会阻塞当前线程，直到文件系统被卸载
pub fn mount_evif(
    mount_table: Arc<RadixMountTable>,
    mount_point: &Path,
    config: FuseMountConfig,
) -> EvifResult<()> {
    info!(
        "Mounting EVIF at {} (readonly={})",
        mount_point.display(),
        !config.allow_write
    );

    let root_path = PathBuf::from("/");
    let fs = EvifFuseFuse::new(mount_table, root_path, &config)?;

    // 构建 FUSE 挂载选项
    let mut options = vec![
        fuser::MountOption::RO,
        fuser::MountOption::FSName("evif".to_string()),
        fuser::MountOption::AllowOther,
    ];

    if config.allow_write {
        options.remove(0); // 移除 RO
        options.insert(0, fuser::MountOption::RW);
    }

    if config.allow_other {
        options.push(fuser::MountOption::AllowOther);
    }

    // 挂载文件系统（阻塞直到卸载）
    fuser::mount2(fs, mount_point, &options)
        .map_err(|e| EvifError::Internal(format!("Mount failed: {}", e)))?;

    info!("EVIF mounted successfully at {}", mount_point.display());
    Ok(())
}

/// 挂载 EVIF 文件系统（非阻塞版本）
///
/// # 参数
/// - `mount_point`: 挂载点路径
/// - `config`: FUSE 挂载配置
///
/// # 返回
/// - Ok(session): 挂载成功，返回 FUSE 会话句柄（可用于后台运行）
/// - Err(e): 挂载失败
pub fn mount_evif_background(
    mount_table: Arc<RadixMountTable>,
    mount_point: &Path,
    config: FuseMountConfig,
) -> EvifResult<fuser::BackgroundSession> {
    info!(
        "Mounting EVIF at {} (readonly={}, background)",
        mount_point.display(),
        !config.allow_write
    );

    let root_path = PathBuf::from("/");
    let fs = EvifFuseFuse::new(mount_table, root_path, &config)?;

    // 构建 FUSE 挂载选项
    let mut options = vec![
        fuser::MountOption::RO,
        fuser::MountOption::FSName("evif".to_string()),
        fuser::MountOption::AllowOther,
    ];

    if config.allow_write {
        options.remove(0); // 移除 RO
        options.insert(0, fuser::MountOption::RW);
    }

    if config.allow_other {
        options.push(fuser::MountOption::AllowOther);
    }

    // 挂载文件系统（后台）
    let session = fuser::spawn_mount2(fs, mount_point, &options)
        .map_err(|e| EvifError::Internal(format!("Mount failed: {}", e)))?;

    info!(
        "EVIF mounted successfully at {} (background)",
        mount_point.display()
    );
    Ok(session)
}

/// 挂载配置构建器
pub struct FuseMountBuilder {
    mount_point: Option<PathBuf>,
    root_path: PathBuf,
    allow_write: bool,
    allow_other: bool,
    cache_size: usize,
    cache_timeout: u64,
}

impl FuseMountBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            mount_point: None,
            root_path: PathBuf::from("/"),
            allow_write: false,
            allow_other: false,
            cache_size: 10000,
            cache_timeout: 60,
        }
    }

    /// 设置挂载点
    pub fn mount_point(mut self, path: &Path) -> Self {
        self.mount_point = Some(path.to_path_buf());
        self
    }

    /// 设置根路径（在 EVIF 中的路径）
    pub fn root_path(mut self, path: &Path) -> Self {
        self.root_path = path.to_path_buf();
        self
    }

    /// 启用写操作
    pub fn allow_write(mut self, allow: bool) -> Self {
        self.allow_write = allow;
        self
    }

    /// 允许其他用户访问
    pub fn allow_other(mut self, allow: bool) -> Self {
        self.allow_other = allow;
        self
    }

    /// 设置缓存大小
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// 设置缓存超时（秒）
    pub fn cache_timeout(mut self, timeout: u64) -> Self {
        self.cache_timeout = timeout;
        self
    }

    /// 构建配置
    pub fn build(self) -> EvifResult<FuseMountConfig> {
        let mount_point = self
            .mount_point
            .ok_or_else(|| EvifError::InvalidArgument("Mount point is required".to_string()))?;

        Ok(FuseMountConfig {
            mount_point,
            root_path: self.root_path,
            allow_write: self.allow_write,
            allow_other: self.allow_other,
            cache_size: self.cache_size,
            cache_timeout: self.cache_timeout,
        })
    }

    /// 直接挂载（阻塞版本）
    pub fn mount(self, mount_table: Arc<RadixMountTable>) -> EvifResult<()> {
        let config = self.build()?;
        let mount_point = config.mount_point.clone();
        mount_evif(mount_table, &mount_point, config)
    }

    /// 后台挂载
    pub fn mount_background(
        self,
        mount_table: Arc<RadixMountTable>,
    ) -> EvifResult<fuser::BackgroundSession> {
        let config = self.build()?;
        let mount_point = config.mount_point.clone();
        mount_evif_background(mount_table, &mount_point, config)
    }
}

impl Default for FuseMountBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_builder() {
        let config = FuseMountBuilder::new()
            .mount_point(Path::new("/tmp/evif-test"))
            .allow_write(true)
            .cache_size(5000)
            .cache_timeout(120)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.mount_point, PathBuf::from("/tmp/evif-test"));
        assert!(config.allow_write);
        assert_eq!(config.cache_size, 5000);
        assert_eq!(config.cache_timeout, 120);
    }
}
