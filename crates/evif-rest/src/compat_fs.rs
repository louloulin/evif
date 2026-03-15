// 前端兼容 API：/api/v1/fs/*
//
// evif-web 早期实现使用 /api/v1/fs/list|read|write|create|delete
// 这里用最小适配把请求转到现有的 EvifPlugin 接口，并返回前端期望的字段形状。

use crate::{AppState, RestError, RestResult};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FsQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct FsWriteBody {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct FsReadResponse {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct FsListResponse {
    pub nodes: Vec<FsNode>,
}

#[derive(Debug, Serialize)]
pub struct FsNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FsNode>>,
}

fn join_path(dir: &str, name: &str) -> String {
    if dir == "/" {
        format!("/{}", name)
    } else if dir.ends_with('/') {
        format!("{}{}", dir, name)
    } else {
        format!("{}/{}", dir, name)
    }
}

pub struct CompatFsHandlers;

impl CompatFsHandlers {
    /// 列出目录内容（支持路径翻译）
    ///
    /// Task 05: 使用 lookup_with_path 替代 lookup，支持挂载点路径翻译
    ///
    /// ## 路径翻译机制
    /// - 输入路径 "/mem/world" → 查找插件 "/mem" + 相对路径 "/world"
    /// - 输入路径 "/" → 特殊处理，返回所有挂载点列表
    ///
    /// ## 根路径特殊处理
    /// 根路径 "/" 没有对应的插件，因此返回挂载点列表作为顶层目录。
    /// 这样 UI 可以显示类似文件系统的目录结构：
    /// ```text
    /// /                    # 根路径：显示挂载点
    /// ├── /mem             # 挂载点
    /// ├── /hello           # 挂载点
    /// └── /local           # 挂载点
    /// ```
    pub async fn list(State(state): State<AppState>, Query(q): Query<FsQuery>) -> RestResult<Json<FsListResponse>> {
        // Task 05: 使用 lookup_with_path 替代 lookup，支持路径翻译
        // 返回 (插件选项, 相对路径)，例如：/mem/world → (Some(mem插件), "/world")
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&q.path).await;

        // 根路径特殊处理：返回挂载点作为顶层目录
        // 当 relative_path == "/" 且 plugin_opt.is_none() 时，说明请求的是根路径
        if relative_path == "/" && plugin_opt.is_none() {
            let mounts = state.mount_table.list_mounts().await;
            let nodes = mounts
                .into_iter()
                .map(|m| FsNode {
                    path: m.clone(),
                    name: m.trim_start_matches('/').to_string(),
                    is_dir: true,
                    children: None,
                })
                .collect();
            return Ok(Json(FsListResponse { nodes }));
        }

        // 非根路径：使用相对路径调用插件
        // 例如：请求 /mem/world，relative_path 为 /world，调用 mem_fs.readdir("/world")
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", q.path)))?;

        let entries = plugin
            .readdir(&relative_path)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        let nodes = entries
            .into_iter()
            .map(|e| FsNode {
                // Use original request path (q.path) not relative_path to build correct full paths
                // Example: /hello + hello = /hello/hello (not /hello)
                path: join_path(&q.path, &e.name),
                name: e.name,
                is_dir: e.is_dir,
                children: None,
            })
            .collect();

        Ok(Json(FsListResponse { nodes }))
    }

    /// 读取文件内容（兼容旧版前端 API）
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件进行读取
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.read("/nested/test.txt", 0, 0)`
    ///
    /// # 相关方法
    /// - [`RadixMountTable::lookup_with_path()`]: VFS 路径翻译实现
    pub async fn read(State(state): State<AppState>, Query(q): Query<FsQuery>) -> RestResult<Json<FsReadResponse>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&q.path).await;
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", q.path)))?;

        let data = plugin
            .read(&relative_path, 0, 0)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        let content =
            String::from_utf8(data).map_err(|e| RestError::Internal(format!("Invalid UTF-8: {}", e)))?;

        Ok(Json(FsReadResponse { content }))
    }

    /// 写入文件内容（支持路径翻译）
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件写入文件内容
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.write("/nested/test.txt", data, 0, TRUNCATE)`
    pub async fn write(
        State(state): State<AppState>,
        Query(q): Query<FsQuery>,
        Json(body): Json<FsWriteBody>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&q.path).await;
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", q.path)))?;

        let _bytes = plugin
            .write(&relative_path, body.content.into_bytes(), 0, evif_core::WriteFlags::TRUNCATE)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({ "ok": true })))
    }

    /// 创建文件（支持路径翻译）
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件创建文件
    /// 3. 返回创建成功的消息
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/new/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/new/test.txt")`
    /// - 插件调用: `mem_plugin.create("/new/test.txt", 0o644)`
    pub async fn create(
        State(state): State<AppState>,
        Json(body): Json<FsQuery>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&body.path).await;
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", body.path)))?;

        plugin
            .create(&relative_path, 0o644)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({ "ok": true })))
    }

    pub async fn delete(State(state): State<AppState>, Query(q): Query<FsQuery>) -> RestResult<Json<serde_json::Value>> {
        let plugin = state
            .mount_table
            .lookup(&q.path)
            .await
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", q.path)))?;

        plugin
            .remove(&q.path)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({ "ok": true })))
    }
}

