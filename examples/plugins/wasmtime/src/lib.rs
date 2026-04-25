// EVIF Wasmtime Example Plugin
//
// 纯 Rust WASM 插件示例（不使用 Extism PDK）
// 适用于 Wasmtime Component Model 后端
//
// 编译方式:
//   cargo build --target wasm32-unknown-unknown --release
//   生成的 .wasm 文件可直接被 evif-core Wasmtime 后端加载
//
// 调用方式:
//   所有函数接受 JSON 字符串输入，返回 JSON 字符串输出
//   二进制数据使用 Base64 编码

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============== 数据结构 ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    name: String,
    size: u64,
    mode: u32,
    modified: String,
    is_dir: bool,
}

#[derive(Debug, Serialize)]
struct SuccessResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReadResponse {
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct WriteResponse {
    bytes_written: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReaddirResponse {
    files: Vec<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatResponse {
    file: Option<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ============== 内存文件系统 ==============

// 线程局部存储的内存文件系统
//
// WASM 模块是单线程的，所以 `thread_local!` 足够安全
thread_local! {
    static FS: std::cell::RefCell<FileSystem> = std::cell::RefCell::new(FileSystem::new());
}

struct FileSystem {
    files: HashMap<String, Vec<u8>>,
    dirs: HashMap<String, u32>, // path -> mode
}

impl FileSystem {
    fn new() -> Self {
        let mut dirs = HashMap::new();
        dirs.insert("/".to_string(), 0o755);
        Self {
            files: HashMap::new(),
            dirs,
        }
    }
}

fn current_timestamp() -> String {
    "2026-03-31T00:00:00Z".to_string()
}

fn extract_name(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or("unknown")
        .to_string()
}

fn json_err(msg: &str) -> String {
    serde_json::to_string(&SuccessResponse {
        success: false,
        error: Some(msg.to_string()),
    })
    .unwrap()
}

fn json_ok() -> String {
    serde_json::to_string(&SuccessResponse {
        success: true,
        error: None,
    })
    .unwrap()
}

// ============== 导出函数 ==============

// 导出的 WASM 函数需要使用 no_mangle
// Wasmtime 后端通过函数名查找并调用

#[no_mangle]
pub extern "C" fn evif_create(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
        #[allow(dead_code)]
        perm: u32,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();
        if let std::collections::hash_map::Entry::Vacant(entry) = fs.files.entry(req.path) {
            entry.insert(Vec::new());
            encode_response(&json_ok())
        } else {
            let resp = json_err("File already exists");
            encode_response(&resp)
        }
    })
}

#[no_mangle]
pub extern "C" fn evif_mkdir(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
        perm: u32,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();
        if let std::collections::hash_map::Entry::Vacant(entry) = fs.dirs.entry(req.path) {
            entry.insert(req.perm);
            encode_response(&json_ok())
        } else {
            let resp = json_err("Directory already exists");
            encode_response(&resp)
        }
    })
}

#[no_mangle]
pub extern "C" fn evif_read(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
        offset: u64,
        size: u64,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let fs = fs.borrow();
        match fs.files.get(&req.path) {
            Some(data) => {
                let start = req.offset as usize;
                let end = if req.size == 0 {
                    data.len()
                } else {
                    (start + req.size as usize).min(data.len())
                };
                let sliced = if start < data.len() {
                    &data[start..end]
                } else {
                    &[]
                };
                let encoded = base64_encode(sliced);
                let resp = serde_json::to_string(&ReadResponse {
                    data: encoded,
                    error: None,
                })
                .unwrap();
                encode_response(&resp)
            }
            None => {
                let resp = json_err("File not found");
                encode_response(&resp)
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn evif_write(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
        data: String,
        offset: i64,
        #[allow(dead_code)]
        flags: u32,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    let new_data = match base64_decode(&req.data) {
        Ok(d) => d,
        Err(e) => {
            let resp = json_err(&format!("Base64 decode failed: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();
        let written = new_data.len() as u64;

        match fs.files.get_mut(&req.path) {
            Some(existing) => {
                if req.offset >= 0 {
                    let offset = req.offset as usize;
                    if offset >= existing.len() {
                        existing.extend_from_slice(&new_data);
                    } else {
                        let end = (offset + new_data.len()).min(existing.len());
                        existing[offset..end].copy_from_slice(&new_data[..end - offset]);
                        if offset + new_data.len() > existing.len() {
                            existing.extend_from_slice(&new_data[existing.len() - offset..]);
                        }
                    }
                } else {
                    // Append
                    existing.extend_from_slice(&new_data);
                }
            }
            None => {
                fs.files.insert(req.path, new_data);
            }
        }

        let resp = serde_json::to_string(&WriteResponse {
            bytes_written: written,
            error: None,
        })
        .unwrap();
        encode_response(&resp)
    })
}

#[no_mangle]
pub extern "C" fn evif_readdir(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let fs = fs.borrow();
        let prefix = req.path.trim_end_matches('/');
        let mut files = Vec::new();

        // List files in this directory
        for (path, data) in &fs.files {
            if let Some(rel) = path.strip_prefix(&format!("{}/", prefix.trim_start_matches('/'))) {
                if !rel.contains('/') {
                    files.push(FileInfo {
                        name: extract_name(path),
                        size: data.len() as u64,
                        mode: 0o644,
                        modified: current_timestamp(),
                        is_dir: false,
                    });
                }
            }
        }

        // List subdirectories
        for (path, &mode) in &fs.dirs {
            if let Some(rel) = path.strip_prefix(&format!("{}/", prefix.trim_start_matches('/'))) {
                if !rel.contains('/') && !rel.is_empty() {
                    files.push(FileInfo {
                        name: extract_name(path),
                        size: 0,
                        mode,
                        modified: current_timestamp(),
                        is_dir: true,
                    });
                }
            }
        }

        let resp = serde_json::to_string(&ReaddirResponse { files, error: None }).unwrap();
        encode_response(&resp)
    })
}

#[no_mangle]
pub extern "C" fn evif_stat(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let fs = fs.borrow();

        // Check if it's a file
        if let Some(data) = fs.files.get(&req.path) {
            let resp = serde_json::to_string(&StatResponse {
                file: Some(FileInfo {
                    name: extract_name(&req.path),
                    size: data.len() as u64,
                    mode: 0o644,
                    modified: current_timestamp(),
                    is_dir: false,
                }),
                error: None,
            })
            .unwrap();
            return encode_response(&resp);
        }

        // Check if it's a directory
        if let Some(&mode) = fs.dirs.get(&req.path) {
            let resp = serde_json::to_string(&StatResponse {
                file: Some(FileInfo {
                    name: extract_name(&req.path),
                    size: 0,
                    mode,
                    modified: current_timestamp(),
                    is_dir: true,
                }),
                error: None,
            })
            .unwrap();
            return encode_response(&resp);
        }

        let resp = json_err("File not found");
        encode_response(&resp)
    })
}

#[no_mangle]
pub extern "C" fn evif_remove(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();
        let removed_file = fs.files.remove(&req.path).is_some();
        let removed_dir = fs.dirs.remove(&req.path).is_some();

        if removed_file || removed_dir {
            encode_response(&json_ok())
        } else {
            let resp = json_err("File not found");
            encode_response(&resp)
        }
    })
}

#[no_mangle]
pub extern "C" fn evif_rename(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        old_path: String,
        new_path: String,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();

        // Try file rename
        if let Some(data) = fs.files.remove(&req.old_path) {
            fs.files.insert(req.new_path, data);
            return encode_response(&json_ok());
        }

        // Try dir rename
        if let Some(mode) = fs.dirs.remove(&req.old_path) {
            fs.dirs.insert(req.new_path, mode);
            return encode_response(&json_ok());
        }

        let resp = json_err("Source file not found");
        encode_response(&resp)
    })
}

#[no_mangle]
pub extern "C" fn evif_remove_all(input_ptr: u32, input_len: u32) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
        String::from_utf8_lossy(slice).to_string()
    };

    #[derive(Deserialize)]
    struct Request {
        path: String,
    }

    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = json_err(&format!("Invalid JSON: {}", e));
            return encode_response(&resp);
        }
    };

    FS.with(|fs| {
        let mut fs = fs.borrow_mut();
        let prefix = req.path.trim_end_matches('/');
        fs.files.retain(|k, _| !k.starts_with(prefix));
        fs.dirs.retain(|k, _| !k.starts_with(prefix));
        encode_response(&json_ok())
    })
}

// ============== 辅助函数 ==============

/// Base64 编码（内联实现，不依赖外部 crate）
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Base64 解码（内联实现）
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);

    let mut i = 0;
    while i < input.len() {
        let c0 = input.chars().nth(i).unwrap();
        let c1 = input.chars().nth(i + 1).unwrap();
        let v0 = TABLE.get(c0 as usize).copied().unwrap_or(-1);
        let v1 = TABLE.get(c1 as usize).copied().unwrap_or(-1);

        if v0 < 0 || v1 < 0 {
            return Err("Invalid base64 character".to_string());
        }

        result.push(((v0 as u8) << 2) | ((v1 as u8) >> 4));

        if i + 2 < input.len() {
            let c2 = input.chars().nth(i + 2).unwrap();
            let v2 = TABLE.get(c2 as usize).copied().unwrap_or(-1);
            if v2 < 0 {
                return Err("Invalid base64 character".to_string());
            }
            result.push(((v1 as u8) << 4) | ((v2 as u8) >> 2));

            if i + 3 < input.len() {
                let c3 = input.chars().nth(i + 3).unwrap();
                let v3 = TABLE.get(c3 as usize).copied().unwrap_or(-1);
                if v3 < 0 {
                    return Err("Invalid base64 character".to_string());
                }
                result.push(((v2 as u8) << 6) | (v3 as u8));
            }
        }

        i += 4;
    }

    Ok(result)
}

/// 将 JSON 响应字符串编码为 u64 (ptr: u32, len: u32)
/// 在 WASM 中，我们通过线性内存传递字符串
fn encode_response(json: &str) -> u64 {
    let len = json.len() as u32;
    let ptr = json.as_ptr() as u32;
    ((len as u64) << 32) | (ptr as u64)
}
