//! EVIF 存储后端示例
//!
//! 演示如何使用不同的存储后端（内存、Sled、RocksDB）

use evif_graph::{Graph, Node, NodeType};
use evif_storage::{MemoryStorage, SledStorage, StorageBackend};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF 存储后端示例 ===\n");

    // 示例1: 内存存储
    println!("1. 内存存储后端:");
    let memory_storage = MemoryStorage::new();
    let graph1 = Graph::with_storage(memory_storage);
    demo_graph("内存", &graph1).await?;
    println!();

    // 示例2: Sled嵌入式数据库
    println!("2. Sled嵌入式数据库后端:");
    let sled_path = PathBuf::from("/tmp/evif_sled_example");
    let _ = std::fs::remove_dir_all(&sled_path); // 清理旧数据

    let sled_storage = SledStorage::new(&sled_path).await?;
    let graph2 = Graph::with_storage(sled_storage);
    demo_graph("Sled", &graph2).await?;
    println!();

    // 清理Sled数据
    let _ = std::fs::remove_dir_all(&sled_path);

    // 示例3: RocksDB（如果启用）
    #[cfg(feature = "rocksdb-backend")]
    {
        println!("3. RocksDB高性能存储后端:");
        let rocksdb_path = PathBuf::from("/tmp/evif_rocksdb_example");
        let _ = std::fs::remove_dir_all(&rocksdb_path);

        use evif_storage::RocksDBStorage;
        let rocksdb_storage = RocksDBStorage::new(&rocksdb_path)?;
        let graph3 = Graph::with_storage(rocksdb_storage);
        demo_graph("RocksDB", &graph3).await?;

        let _ = std::fs::remove_dir_all(&rocksdb_path);
        println!();
    }

    println!("=== 示例完成 ===");
    Ok(())
}

async fn demo_graph(name: &str, graph: &Graph) -> Result<(), Box<dyn std::error::Error>> {
    // 创建测试节点
    let node1 = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::File,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), format!("{}_file1.txt", name).into()),
            ("size".to_string(), 100u64.into()),
        ].into(),
        content_handle: None,
    };

    let node2 = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), format!("{}_dir", name).into()),
        ].into(),
        content_handle: None,
    };

    // 添加节点
    graph.add_node(node1).await?;
    graph.add_node(node2).await?;

    // 显示统计
    println!("  ✓ 使用{}存储", name);
    println!("  - 节点数: {}", graph.node_count().await);
    println!("  - 边数: {}", graph.edge_count().await);

    Ok(())
}
