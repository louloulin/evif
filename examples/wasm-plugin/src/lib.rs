// EVIF Example WASM Plugin
//
// 使用 Extism PDK 实现的键值存储插件示例

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    data: String,  // Base64 编码
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
    file: FileInfo,
    error: Option<String>,
}

// ============== 全局状态 ==============//

/// 使用 Extism 的 KV 存储来持久化数据
///
/// 键格式: "file:<path>"
/// 值格式: JSON { data: base64, modified: timestamp }

/// 从路径获取 KV 键
fn get_kv_key(path: &str) -> String {
    format!("file:{}", path)
}

/// 解析路径中的文件名
fn extract_name(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .last()
        .unwrap_or("unknown")
        .to_string()
}

/// 获取当前时间戳
fn current_timestamp() -> String {
    // 简化的时间戳（实际应使用 WASI 时钟）
    chrono::Utc::now().to_rfc3339()
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

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 创建空文件
    let file_data = serde_json::json!({
        "data": "",
        "modified": current_timestamp(),
        "mode": req.perm,
        "is_dir": false
    });

    var::set(&get_kv_key(&req.path), file_data.to_string())
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let response = SuccessResponse {
        success: true,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 创建目录
#[plugin_fn]
pub fn evif_mkdir(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        perm: u32,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 创建目录标记
    let dir_data = serde_json::json!({
        "data": "",
        "modified": current_timestamp(),
        "mode": req.perm,
        "is_dir": true
    });

    var::set(&get_kv_key(&req.path), dir_data.to_string())
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let response = SuccessResponse {
        success: true,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
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

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 从 KV 存储读取
    let key = get_kv_key(&req.path);
    let file_data = var::get(&key)
        .map_err(|e| format!("File not found: {}", e))?;

    let data: serde_json::Value = serde_json::from_str(&file_data)
        .map_err(|e| format!("Corrupted file data: {}", e))?;

    let base64_data = data["data"].as_str().unwrap_or("");

    // 解码并应用 offset/size
    let full_data = base64::decode(base64_data)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    let start = req.offset as usize;
    let end = if req.size == 0 {
        full_data.len()
    } else {
        (req.offset + req.size) as usize
    };

    let sliced_data = &full_data[start..end.min(full_data.len())];
    let encoded = base64::encode(sliced_data);

    let response = ReadResponse {
        data: encoded,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 写入文件
#[plugin_fn]
pub fn evif_write(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
        data: String,  // Base64 编码
        offset: i64,
        flags: u32,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 解码数据
    let new_data = base64::decode(&req.data)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    let key = get_kv_key(&req.path);

    // 读取现有数据（如果存在）
    let existing_data = var::get(&key).ok();
    let mut final_data = new_data;

    if let Some(existing) = existing_data {
        if let Ok(data_json) = serde_json::from_str::<serde_json::Value>(&existing) {
            if let Some(existing_base64) = data_json["data"].as_str() {
                if req.offset >= 0 {
                    // 追加或覆盖写入
                    let mut decoded = base64::decode(existing_base64).unwrap_or_default();
                    if req.offset as usize >= decoded.len() {
                        // 追加
                        decoded.extend_from_slice(&new_data);
                    } else {
                        // 覆盖
                        let offset = req.offset as usize;
                        let end = (offset + new_data.len()).min(decoded.len());
                        decoded[offset..end].copy_from_slice(&new_data);
                        if offset + new_data.len() > decoded.len() {
                            decoded.extend_from_slice(&new_data[decoded.len() - offset..]);
                        }
                    }
                    final_data = decoded;
                }
            }
        }
    }

    // 编码并保存
    let encoded = base64::encode(&final_data);
    let file_data = serde_json::json!({
        "data": encoded,
        "modified": current_timestamp(),
        "mode": 0,
        "is_dir": false
    });

    var::set(&key, file_data.to_string())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let response = WriteResponse {
        bytes_written: final_data.len() as u64,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 列出目录
#[plugin_fn]
pub fn evif_readdir(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // 获取所有键
    let all_keys = var::keys()
        .map_err(|e| format!("Failed to list keys: {}", e))?;

    // 筛选属于该目录的文件
    let prefix = format!("file:{}", req.path.trim_end_matches('/'));
    let mut files = Vec::new();

    for key in all_keys {
        if key.starts_with(&prefix) {
            if let Ok(file_data) = var::get(&key) {
                if let Ok(data_json) = serde_json::from_str::<serde_json::Value>(&file_data) {
                    let name = extract_name(&key);
                    let size = data_json["data"]
                        .as_str()
                        .map(|s| s.len() as u64)
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

    let response = ReaddirResponse {
        files,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 获取文件信息
#[plugin_fn]
pub fn evif_stat(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let key = get_kv_key(&req.path);
    let file_data = var::get(&key)
        .map_err(|e| format!("File not found: {}", e))?;

    let data_json: serde_json::Value = serde_json::from_str(&file_data)
        .map_err(|e| format!("Corrupted file data: {}", e))?;

    let name = extract_name(&req.path);
    let size = data_json["data"]
        .as_str()
        .map(|s| s.len() as u64)
        .unwrap_or(0);
    let mode = data_json["mode"].as_u64().unwrap_or(0) as u32;
    let modified = data_json["modified"].as_str().unwrap_or("").to_string();
    let is_dir = data_json["is_dir"].as_bool().unwrap_or(false);

    let response = StatResponse {
        file: FileInfo {
            name,
            size,
            mode,
            modified,
            is_dir,
        },
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 删除文件
#[plugin_fn]
pub fn evif_remove(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let key = get_kv_key(&req.path);
    var::remove(&key)
        .map_err(|e| format!("Failed to remove file: {}", e))?;

    let response = SuccessResponse {
        success: true,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 重命名文件
#[plugin_fn]
pub fn evif_rename(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        old_path: String,
        new_path: String,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let old_key = get_kv_key(&req.old_path);
    let new_key = get_kv_key(&req.new_path);

    // 读取旧文件
    let file_data = var::get(&old_key)
        .map_err(|e| format!("Source file not found: {}", e))?;

    // 写入新位置
    var::set(&new_key, &file_data)
        .map_err(|e| format!("Failed to create new file: {}", e))?;

    // 删除旧文件
    var::remove(&old_key)
        .map_err(|e| format!("Failed to remove old file: {}", e))?;

    let response = SuccessResponse {
        success: true,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}

/// 递归删除目录
#[plugin_fn]
pub fn evif_remove_all(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = serde_json::from_str(&input)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let prefix = format!("file:{}", req.path.trim_end_matches('/'));
    let all_keys = var::keys()
        .map_err(|e| format!("Failed to list keys: {}", e))?;

    // 删除所有匹配的键
    let mut removed = 0;
    for key in all_keys {
        if key.starts_with(&prefix) {
            var::remove(&key)?;
            removed += 1;
        }
    }

    let response = SuccessResponse {
        success: true,
        error: None,
    };

    Ok(serde_json::to_string(&response).unwrap())
}
