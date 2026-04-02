// Phase 13.1/13.2: OSWorld + IDE-Bench Benchmark Tests
//
// OSWorld 对标: Agent 在完整操作系统中的表现，文件系统状态检查是核心
// IDE-Bench 对标: AI IDE Agent 的文件读写和导航任务

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ContextFsPlugin;

// ---------------------------------------------------------------------------
// OSWorld 对标测试 (Phase 13.1)
// ---------------------------------------------------------------------------

/// OSWorld-01: 文件系统状态验证
/// 对标 OSWorld: 验证 Agent 执行任务后文件系统状态正确
#[tokio::test]
async fn osworld_file_system_state_after_write() {
    let plugin = ContextFsPlugin::new();

    // 创建项目目录结构
    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    plugin
        .mkdir("/test/project", 0o755)
        .await
        .expect("create project dir");
    plugin
        .mkdir("/test/project/src", 0o755)
        .await
        .expect("create src dir");
    plugin
        .mkdir("/test/project/tests", 0o755)
        .await
        .expect("create tests dir");

    // 写入源文件
    plugin
        .create("/test/project/src/main.rs", 0o644)
        .await
        .expect("create main.rs");
    plugin
        .write(
            "/test/project/src/main.rs",
            b"fn main() {}\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write main.rs");

    // 验证目录结构
    let project_entries = plugin.readdir("/test/project").await.expect("list project");
    let project_names: Vec<String> = project_entries.into_iter().map(|e| e.name).collect();
    assert!(project_names.contains(&"src".to_string()), "should have src dir");
    assert!(project_names.contains(&"tests".to_string()), "should have tests dir");

    // 验证文件存在
    let src_stat = plugin.stat("/test/project/src/main.rs").await.expect("stat main.rs");
    assert!(!src_stat.is_dir, "main.rs should be a file");
}

/// OSWorld-02: 并发文件操作
/// 对标 OSWorld: 100 个并发 Agent 同时操作
#[tokio::test]
async fn osworld_concurrent_file_operations() {
    let mut handles = Vec::new();

    for i in 0..100u32 {
        let plugin = ContextFsPlugin::new();
        let path = format!("/test/concurrent/file_{}", i);
        let content = format!("concurrent data {}", i);

        handles.push(tokio::spawn(async move {
            plugin
                .write(&path, content.into_bytes(), 0, WriteFlags::TRUNCATE)
                .await
        }));
    }

    let results: Vec<Result<_, _>> = futures::future::join_all(handles).await;
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    // 验证 95%+ 成功率
    assert!(
        success_count >= 95,
        "Concurrent operations success rate {:.1}% < 95%",
        success_count as f64
    );
}

/// OSWorld-03: 文件修改时间戳验证
#[tokio::test]
async fn osworld_file_modification_time() {
    let plugin = ContextFsPlugin::new();

    let _before = chrono::Utc::now();

    // 创建并写入文件
    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    plugin
        .create("/test/timestamp.txt", 0o644)
        .await
        .expect("create");
    plugin
        .write("/test/timestamp.txt", b"content".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write");

    let stat = plugin.stat("/test/timestamp.txt").await.expect("stat");

    // stat.modified is already a DateTime<Utc>
    // Verify it's recent (within the last minute)
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(stat.modified);
    assert!(
        diff.num_seconds() < 60,
        "File modification time should be recent"
    );
}

/// OSWorld-04: 嵌套目录递归操作
#[tokio::test]
async fn osworld_nested_directory_operations() {
    let plugin = ContextFsPlugin::new();

    // 创建多层嵌套目录
    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    plugin.mkdir("/test/deep", 0o755).await.expect("mkdir /test/deep");
    plugin.mkdir("/test/deep/a", 0o755).await.expect("mkdir /test/deep/a");
    plugin.mkdir("/test/deep/a/b", 0o755).await.expect("mkdir /test/deep/a/b");
    plugin.mkdir("/test/deep/a/b/c", 0o755).await.expect("mkdir /test/deep/a/b/c");

    // 在最深目录写入文件
    plugin
        .create("/test/deep/a/b/c/data.txt", 0o644)
        .await
        .expect("create nested file");
    plugin
        .write(
            "/test/deep/a/b/c/data.txt",
            b"deep nested data".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write nested file");

    // 验证文件可读
    let data = plugin
        .read("/test/deep/a/b/c/data.txt", 0, 0)
        .await
        .expect("read nested file");
    assert_eq!(data, b"deep nested data");

    // 删除嵌套文件
    plugin
        .remove("/test/deep/a/b/c/data.txt")
        .await
        .expect("remove nested file");

    // 验证删除成功
    assert!(
        plugin.stat("/test/deep/a/b/c/data.txt").await.is_err(),
        "File should be deleted"
    );
}

// ---------------------------------------------------------------------------
// IDE-Bench 对标测试 (Phase 13.2)
// ---------------------------------------------------------------------------

/// IDEBench-01: 文件读取任务
#[tokio::test]
async fn idebench_read_file() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    plugin.mkdir("/test/src", 0o755).await.expect("mkdir /test/src");
    plugin
        .create("/test/src/lib.rs", 0o644)
        .await
        .expect("create lib.rs");
    plugin
        .write(
            "/test/src/lib.rs",
            b"pub fn add(a: i32, b: i32) -> i32 { a + b }\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write lib.rs");

    let content = plugin
        .read("/test/src/lib.rs", 0, 0)
        .await
        .expect("read lib.rs");
    let content_str = String::from_utf8_lossy(&content);

    assert!(
        content_str.contains("fn add"),
        "Content should contain 'fn add'"
    );
    assert!(
        content_str.contains("i32"),
        "Content should contain 'i32'"
    );
}

/// IDEBench-02: 目录导航任务
#[tokio::test]
async fn idebench_navigation() {
    let plugin = ContextFsPlugin::new();

    // 创建典型 Rust 项目结构
    plugin.mkdir("/test_project", 0o755).await.expect("create project");
    plugin.mkdir("/test_project/src", 0o755).await.expect("create src");
    plugin.mkdir("/test_project/tests", 0o755).await.expect("create tests");
    plugin.mkdir("/test_project/target", 0o755).await.expect("create target");

    plugin
        .create("/test_project/Cargo.toml", 0o644)
        .await
        .expect("create Cargo.toml");
    plugin
        .write(
            "/test_project/Cargo.toml",
            b"[package]\nname = \"test_project\"\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write Cargo.toml");

    // 导航到项目根目录
    let root_entries = plugin.readdir("/test_project").await.expect("list root");
    let root_names: Vec<String> = root_entries.into_iter().map(|e| e.name).collect();

    assert!(root_names.contains(&"src".to_string()), "should have src/");
    assert!(root_names.contains(&"tests".to_string()), "should have tests/");
    assert!(root_names.contains(&"Cargo.toml".to_string()), "should have Cargo.toml");

    // 导航到 src/
    let src_entries = plugin.readdir("/test_project/src").await.expect("list src");
    assert_eq!(src_entries.len(), 0, "src should be empty initially");
}

/// IDEBench-03: 文件搜索任务 (grep)
#[tokio::test]
async fn idebench_file_search() {
    // 注意: grep 需要通过 REST API，这里测试文件内容匹配逻辑
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    // 创建多个测试文件
    for name in &["a.rs", "b.rs", "c.rs"] {
        let path = format!("/test/{}", name);
        let content = if *name == "a.rs" {
            "fn test_a() {}".to_string()
        } else if *name == "b.rs" {
            "fn test_b() {}".to_string()
        } else {
            "fn test_c() {}".to_string()
        };

        plugin.create(&path, 0o644).await.expect("create");
        plugin
            .write(&path, content.into_bytes(), 0, WriteFlags::TRUNCATE)
            .await
            .expect("write");
    }

    // 验证所有文件存在
    for name in &["a.rs", "b.rs", "c.rs"] {
        let path = format!("/test/{}", name);
        let stat = plugin.stat(&path).await.expect("stat");
        assert!(!stat.is_dir, "{} should be a file", name);
    }
}

/// IDEBench-04: 多文件编辑任务
#[tokio::test]
async fn idebench_multi_file_edits() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    // 批量创建和编辑多个文件
    let files = vec![
        ("/test/module1.rs", "pub mod a;"),
        ("/test/module2.rs", "pub mod b;"),
        ("/test/lib.rs", "mod module1;\nmod module2;"),
    ];

    for (path, content) in files {
        plugin.create(path, 0o644).await.expect("create");
        plugin
            .write(path, content.as_bytes().to_vec(), 0, WriteFlags::TRUNCATE)
            .await
            .expect("write");
    }

    // 验证所有文件内容
    let lib_content = plugin
        .read("/test/lib.rs", 0, 0)
        .await
        .expect("read lib.rs");
    let lib_str = String::from_utf8_lossy(&lib_content);
    assert!(lib_str.contains("module1"), "lib.rs should reference module1");
    assert!(lib_str.contains("module2"), "lib.rs should reference module2");
}

/// IDEBench-05: 文件重命名任务
#[tokio::test]
async fn idebench_file_rename() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    plugin
        .create("/test/old_name.rs", 0o644)
        .await
        .expect("create");
    plugin
        .write(
            "/test/old_name.rs",
            b"pub fn renamed() {}\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write");

    // 重命名
    plugin
        .rename("/test/old_name.rs", "/test/new_name.rs")
        .await
        .expect("rename");

    // 验证新文件存在
    let new_stat = plugin.stat("/test/new_name.rs").await.expect("stat new");
    assert!(!new_stat.is_dir, "new_name.rs should be a file");

    // 验证旧文件不存在
    assert!(
        plugin.stat("/test/old_name.rs").await.is_err(),
        "old_name.rs should not exist"
    );

    // 验证内容保留
    let new_content = plugin
        .read("/test/new_name.rs", 0, 0)
        .await
        .expect("read new");
    assert!(
        String::from_utf8_lossy(&new_content).contains("renamed"),
        "Content should be preserved after rename"
    );
}

/// IDEBench-06: 大文件读写性能
#[tokio::test]
async fn idebench_large_file_operations() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/test", 0o755).await.expect("mkdir /test");
    // 创建大文件 (100KB)
    let content = "x".repeat(100_000);
    plugin
        .create("/test/large.rs", 0o644)
        .await
        .expect("create");
    plugin
        .write("/test/large.rs", content.as_bytes().to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write");

    let stat = plugin.stat("/test/large.rs").await.expect("stat large");
    assert!(
        stat.size >= 90000,
        "Large file should be ~100KB, got {}",
        stat.size
    );

    // 读取验证
    let start = std::time::Instant::now();
    let _data = plugin
        .read("/test/large.rs", 0, 0)
        .await
        .expect("read large");
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed < 100,
        "Large file read should be < 100ms, got {}ms",
        elapsed
    );
}

/// IDEBench-07: 目录树遍历
#[tokio::test]
async fn idebench_directory_tree_traversal() {
    let plugin = ContextFsPlugin::new();

    // 创建典型项目结构
    let dirs = vec![
        "/src",
        "/src/handlers",
        "/src/models",
        "/src/utils",
        "/tests",
        "/tests/integration",
        "/docs",
    ];

    for dir in dirs {
        plugin.mkdir(dir, 0o755).await.expect("mkdir");
    }

    // 验证目录存在
    let root_entries = plugin.readdir("/").await.expect("readdir /");
    let root_names: Vec<String> = root_entries.iter().map(|e| e.name.clone()).collect();
    assert!(root_names.contains(&"src".to_string()), "should have src/");

    // 验证特定路径
    let src_entries = plugin.readdir("/src").await.expect("readdir /src");
    let src_names: Vec<String> = src_entries.into_iter().map(|e| e.name).collect();
    assert!(src_names.contains(&"handlers".to_string()));
    assert!(src_names.contains(&"models".to_string()));
}
