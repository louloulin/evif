// EVIF Example WASM Plugin
//
// 使用 Extism PDK 实现的键值存储插件示例
// 基于 Extism var 模块实现持久化 KV 存储

use extism_pdk::*;
use serde::{Deserialize, Serialize};

// ============== 数据结构 ==============

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    name: String,
    size: u64,
    mode: u32,
    modified: String,
    is_dir: bool,
}

/// 通用响应
#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
    error: Option<String>,
}

/// 读取响应
#[derive(Debug, Serialize)]
struct ReadResponse {
    data: String, // Base64 编码
    error: Option<String>,
}

/// 写入响应
#[derive(Debug, Serialize)]
struct WriteResponse {
    bytes_written: u64,
    error: Option<String>,
}

/// 列出目录响应
#[derive(Debug, Serialize)]
struct ReaddirResponse {
    files: Vec<FileInfo>,
    error: Option<String>,
}

/// 文件信息响应
#[derive(Debug, Serialize)]
struct StatResponse {
    file: Option<FileInfo>,
    error: Option<String>,
}

// ============== KV 存储辅助 ==============

/// KV 键前缀
const KV_PREFIX: &str = "file:";

/// 从路径获取 KV 键
fn get_kv_key(path: &str) -> String {
    format!("{}{}", KV_PREFIX, path)
}

/// 解析路径中的文件名
fn extract_name(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .next_back()
        .unwrap_or("unknown")
        .to_string()
}

/// 获取当前时间戳（简化版）
fn current_timestamp() -> String {
    // 使用 Extism PDK 的简化时间戳
    // 在 WASM 环境中没有标准时钟，返回固定格式
    "2026-03-31T00:00:00Z".to_string()
}

/// Base64 编码辅助
fn b64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Base64 解码辅助
fn b64_decode(data: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(data)
}

/// 索引键名（跟踪所有已注册的文件路径）
const INDEX_KEY: &str = "__evif_file_index__";

/// 获取文件索引
fn get_file_index() -> Vec<String> {
    var::get::<String>(INDEX_KEY)
        .ok()
        .flatten()
        .map(|s| serde_json::from_str::<Vec<String>>(&s).unwrap_or_default())
        .unwrap_or_default()
}

/// 保存文件索引
fn save_file_index(index: &[String]) -> std::result::Result<(), Error> {
    let json = serde_json::to_string(index).unwrap_or("[]".to_string());
    var::set(INDEX_KEY, &json)
}

/// 添加路径到索引
fn add_to_index(path: &str) {
    let mut index = get_file_index();
    if !index.contains(&path.to_string()) {
        index.push(path.to_string());
        let _ = save_file_index(&index);
    }
}

/// 从索引移除路径
fn remove_from_index(path: &str) {
    let mut index = get_file_index();
    index.retain(|p| !p.starts_with(path));
    let _ = save_file_index(&index);
}

// ============== EVIF 接口实现 ==============

/// 创建文件
#[plugin_fn]
pub fn evif_create(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        perm: u32,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let file_data = serde_json::json!({
        "data": "",
        "modified": current_timestamp(),
        "mode": req.perm,
        "is_dir": false
    });

    var::set(get_kv_key(&req.path), file_data.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;

    add_to_index(&req.path);

    Ok(serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap())
}

/// 创建目录
#[plugin_fn]
pub fn evif_mkdir(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        perm: u32,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let dir_data = serde_json::json!({
        "data": "",
        "modified": current_timestamp(),
        "mode": req.perm,
        "is_dir": true
    });

    var::set(get_kv_key(&req.path), dir_data.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;

    add_to_index(&req.path);

    Ok(serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap())
}

/// 读取文件
#[plugin_fn]
pub fn evif_read(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        offset: u64,
        size: u64,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let key = get_kv_key(&req.path);
    let file_data: String = var::get(&key)
        .map_err(|e| anyhow::anyhow!("KV error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("File not found: {}", req.path))?;

    let data: serde_json::Value = serde_json::from_str(&file_data)
        .map_err(|e| anyhow::anyhow!("Corrupted file data: {}", e))?;

    let base64_data = data["data"].as_str().unwrap_or("");
    let full_data =
        b64_decode(base64_data).map_err(|e| anyhow::anyhow!("Base64 decode failed: {}", e))?;

    let start = req.offset as usize;
    let end = if req.size == 0 {
        full_data.len()
    } else {
        (req.offset as usize + req.size as usize).min(full_data.len())
    };

    let sliced_data = &full_data[start.min(full_data.len())..end];
    let encoded = b64_encode(sliced_data);

    Ok(serde_json::to_string(&ReadResponse {
        data: encoded,
        error: None,
    })
    .unwrap())
}

/// 写入文件
#[plugin_fn]
pub fn evif_write(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        data: String, // Base64 编码
        offset: i64,
        _flags: u32,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let new_data =
        b64_decode(&req.data).map_err(|e| anyhow::anyhow!("Base64 decode failed: {}", e))?;

    let key = get_kv_key(&req.path);

    // 读取现有数据（如果存在）
    let existing_opt: Option<String> = var::get(&key).ok().flatten();

    let mut final_data = new_data.clone();

    if let Some(existing) = existing_opt {
        if let Ok(data_json) = serde_json::from_str::<serde_json::Value>(&existing) {
            if let Some(existing_base64) = data_json["data"].as_str() {
                if req.offset >= 0 {
                    let mut decoded = b64_decode(existing_base64).unwrap_or_default();
                    let offset = req.offset as usize;
                    if offset >= decoded.len() {
                        decoded.extend_from_slice(&new_data);
                    } else {
                        let end = (offset + new_data.len()).min(decoded.len());
                        decoded[offset..end].copy_from_slice(&new_data[..end - offset]);
                        if offset + new_data.len() > decoded.len() {
                            decoded.extend_from_slice(&new_data[decoded.len() - offset..]);
                        }
                    }
                    final_data = decoded;
                }
            }
        }
    }

    let encoded = b64_encode(&final_data);
    let file_data = serde_json::json!({
        "data": encoded,
        "modified": current_timestamp(),
        "mode": 0,
        "is_dir": false
    });

    var::set(&key, file_data.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

    add_to_index(&req.path);

    Ok(serde_json::to_string(&WriteResponse {
        bytes_written: final_data.len() as u64,
        error: None,
    })
    .unwrap())
}

/// 列出目录
#[plugin_fn]
pub fn evif_readdir(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let prefix = req.path.trim_end_matches('/');
    let all_paths = get_file_index();
    let mut files = Vec::new();

    for path in &all_paths {
        if path.starts_with(format!("{}/", prefix).trim_start_matches('/'))
            || path == prefix.trim_start_matches('/')
        {
            let key = get_kv_key(path);
            if let Some(file_data) = var::get::<String>(&key).ok().flatten() {
                if let Ok(data_json) = serde_json::from_str::<serde_json::Value>(&file_data) {
                    let name = extract_name(path);
                    let size = data_json["data"]
                        .as_str()
                        .map(|s: &str| s.len() as u64)
                        .unwrap_or(0);
                    let mode = data_json["mode"].as_u64().unwrap_or(0) as u32;
                    let modified = data_json["modified"].as_str().unwrap_or("").to_string();
                    let is_dir = data_json["is_dir"].as_bool().unwrap_or(false);

                    files.push(FileInfo {
                        name,
                        size,
                        mode,
                        modified,
                        is_dir,
                    });
                }
            }
        }
    }

    Ok(serde_json::to_string(&ReaddirResponse { files, error: None }).unwrap())
}

/// 获取文件信息
#[plugin_fn]
pub fn evif_stat(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let key = get_kv_key(&req.path);
    let file_data: String = var::get(&key)
        .map_err(|e| anyhow::anyhow!("KV error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("File not found: {}", req.path))?;

    let data_json: serde_json::Value = serde_json::from_str(&file_data)
        .map_err(|e| anyhow::anyhow!("Corrupted file data: {}", e))?;

    let name = extract_name(&req.path);
    let size = data_json["data"]
        .as_str()
        .map(|s: &str| s.len() as u64)
        .unwrap_or(0);
    let mode = data_json["mode"].as_u64().unwrap_or(0) as u32;
    let modified = data_json["modified"].as_str().unwrap_or("").to_string();
    let is_dir = data_json["is_dir"].as_bool().unwrap_or(false);

    Ok(serde_json::to_string(&StatResponse {
        file: Some(FileInfo {
            name,
            size,
            mode,
            modified,
            is_dir,
        }),
        error: None,
    })
    .unwrap())
}

/// 删除文件
#[plugin_fn]
pub fn evif_remove(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let key = get_kv_key(&req.path);
    var::remove(&key).map_err(|e| anyhow::anyhow!("Failed to remove file: {}", e))?;

    remove_from_index(&req.path);

    Ok(serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap())
}

/// 重命名文件
#[plugin_fn]
pub fn evif_rename(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        old_path: String,
        new_path: String,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let old_key = get_kv_key(&req.old_path);
    let new_key = get_kv_key(&req.new_path);

    // 读取旧文件
    let file_data: String = var::get(&old_key)
        .map_err(|e| anyhow::anyhow!("KV error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Source file not found: {}", req.old_path))?;

    // 写入新位置
    var::set(&new_key, &file_data)
        .map_err(|e| anyhow::anyhow!("Failed to create new file: {}", e))?;

    // 删除旧文件
    var::remove(&old_key).map_err(|e| anyhow::anyhow!("Failed to remove old file: {}", e))?;

    remove_from_index(&req.old_path);
    add_to_index(&req.new_path);

    Ok(serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap())
}

/// 递归删除目录
#[plugin_fn]
pub fn evif_remove_all(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request =
        serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let prefix = req.path.trim_end_matches('/');
    let all_paths = get_file_index();
    for path in &all_paths {
        if path.starts_with(prefix) {
            let key = get_kv_key(path);
            let _ = var::remove(&key);
        }
    }

    remove_from_index(prefix);

    Ok(serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap())
}
