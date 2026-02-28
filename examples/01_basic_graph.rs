//! EVIF 基础图操作示例
//!
//! 演示如何创建图、添加节点和边、执行基本查询

use evif_graph::{Graph, Node, NodeId, NodeType, Edge, EdgeType};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF 基础图操作示例 ===\n");

    // 1. 创建图实例
    println!("1. 创建图实例...");
    let graph = Graph::new();
    println!("✓ 图创建成功\n");

    // 2. 创建节点
    println!("2. 创建节点...");
    let file_node = Node {
        id: NodeId::new(),
        node_type: NodeType::File,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), "example.txt".into()),
            ("size".to_string(), 1024u64.into()),
        ].into(),
        content_handle: None,
    };

    let dir_node = Node {
        id: NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), "documents".into()),
        ].into(),
        content_handle: None,
    };

    graph.add_node(file_node.clone()).await?;
    graph.add_node(dir_node.clone()).await?;
    println!("✓ 创建了 2 个节点:");
    println!("  - 文件节点: example.txt (1024 bytes)");
    println!("  - 目录节点: documents\n");

    // 3. 创建边（建立关系）
    println!("3. 创建边（建立关系）...");
    let edge = Edge {
        id: Uuid::new_v4(),
        source: dir_node.id,
        target: file_node.id,
        edge_type: EdgeType::Contains,
        weight: None,
        properties: HashMap::new(),
    };

    graph.add_edge(edge).await?;
    println!("✓ 创建边: documents -> example.txt (包含关系)\n");

    // 4. 查询节点
    println!("4. 查询节点...");
    if let Some(node) = graph.get_node(&file_node.id).await? {
        println!("✓ 找到节点:");
        println!("  - ID: {}", node.id);
        println!("  - 类型: {:?}", node.node_type);
        println!("  - 层数: {:?}", node.attributes);
    }
    println!();

    // 5. 查询邻居节点
    println!("5. 查询目录的子节点...");
    let neighbors = graph.neighbors(&dir_node.id).await?;
    println!("✓ 目录 'documents' 包含 {} 个子节点:", neighbors.len());
    for neighbor_id in neighbors {
        if let Some(node) = graph.get_node(&neighbor_id).await? {
            println!("  - {}", node.attributes.get("name").unwrap());
        }
    }
    println!();

    // 6. BFS遍历
    println!("6. BFS遍历图...");
    let visited = graph.bfs_traverse(dir_node.id).await?;
    println!("✓ BFS遍历访问了 {} 个节点", visited.len());
    println!();

    // 7. 统计信息
    println!("7. 图统计信息:");
    println!("  - 总节点数: {}", graph.node_count().await);
    println!("  - 总边数: {}", graph.edge_count().await);
    println!();

    println!("=== 示例完成 ===");
    Ok(())
}
