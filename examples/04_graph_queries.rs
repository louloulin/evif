//! EVIF 图查询和遍历示例
//!
//! 演示高级图查询、路径查找和图分析

use evif_graph::{Graph, Node, NodeType, Edge, EdgeType};
use evif_graph::executor::{QueryExecutor, PathFinder, GraphAnalyzer};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF 图查询和分析示例 ===\n");

    // 1. 创建图并添加测试数据
    println!("1. 创建测试图...");
    let graph = create_test_graph().await?;
    println!("✓ 测试图创建完成");
    println!("  - 节点数: {}", graph.node_count().await);
    println!("  - 边数: {}", graph.edge_count().await);
    println!();

    // 2. 按类型查询节点
    println!("2. 按类型查询节点...");
    let executor = QueryExecutor::new(&graph);

    let files = executor.find_by_type(NodeType::File).await?;
    println!("✓ 找到 {} 个文件节点", files.len());
    for file in &files {
        if let Some(name) = file.attributes.get("name") {
            println!("  - {}", name);
        }
    }
    println!();

    // 3. 最短路径查找
    println!("3. 查找两个节点间的最短路径...");
    let nodes: Vec<_> = graph.nodes().await?;
    if nodes.len() >= 2 {
        let start = nodes[0].id;
        let end = nodes[nodes.len() - 1].id;

        let path = executor.find_shortest_path(start, end).await?;
        if let Some(p) = path {
            println!("✓ 找到路径，长度: {}", p.len());
            for (i, node_id) in p.iter().enumerate() {
                if let Some(node) = graph.get_node(node_id).await? {
                    let name = node.attributes.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    print!("{}", name);
                    if i < p.len() - 1 {
                        print!(" -> ");
                    }
                }
            }
            println!("\n");
        } else {
            println!("✓ 未找到路径\n");
        }
    }

    // 4. 图分析
    println!("4. 图分析...");
    let analyzer = GraphAnalyzer::new(&graph);

    // 密度
    let density = analyzer.density().await?;
    println!("✓ 图密度: {:.4}", density);

    // 连通分量
    let components = analyzer.connected_components().await?;
    println!("✓ 连通分量数: {}", components.len());
    for (i, component) in components.iter().enumerate() {
        println!("  - 分量 {}: {} 个节点", i + 1, component.len());
    }
    println!();

    // 5. BFS遍历
    println!("5. BFS遍历示例...");
    if let Some(root) = nodes.first() {
        let visited = graph.bfs_traverse(root.id).await?;
        println!("✓ BFS从根节点访问了 {} 个节点", visited.len());
    }
    println!();

    // 6. 度中心性分析
    println!("6. 节点度中心性（连接数最多的前3个节点）...");
    let all_nodes = graph.nodes().await?;
    let mut node_degrees: Vec<_> = all_nodes.iter()
        .map(|node| {
            let degree = graph.neighbors(&node.id).await.unwrap_or_default().len();
            (node, degree)
        })
        .collect();

    node_degrees.sort_by(|a, b| b.1.cmp(&a.1));

    for (node, degree) in node_degrees.iter().take(3) {
        let name = node.attributes.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        println!("  - {}: {} 个连接", name, degree);
    }
    println!();

    println!("=== 示例完成 ===");
    Ok(())
}

/// 创建一个复杂的测试图
async fn create_test_graph() -> Result<Graph, Box<dyn std::error::Error>> {
    let graph = Graph::new();

    // 创建节点
    let root = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [("name".to_string(), "root".into())].into(),
        content_handle: None,
    };

    let home = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [("name".to_string(), "home".into())].into(),
        content_handle: None,
    };

    let user = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [("name".to_string(), "user".into())].into(),
        content_handle: None,
    };

    let documents = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [("name".to_string(), "documents".into())].into(),
        content_handle: None,
    };

    let file1 = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::File,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), "readme.txt".into()),
            ("size".to_string(), 1024u64.into()),
        ].into(),
        content_handle: None,
    };

    let file2 = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::File,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), "code.rs".into()),
            ("size".to_string(), 2048u64.into()),
        ].into(),
        content_handle: None,
    };

    let downloads = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::Directory,
        metadata: HashMap::new(),
        attributes: [("name".to_string(), "downloads".into())].into(),
        content_handle: None,
    };

    let file3 = Node {
        id: evif_graph::NodeId::new(),
        node_type: NodeType::File,
        metadata: HashMap::new(),
        attributes: [
            ("name".to_string(), "archive.zip".into()),
            ("size".to_string(), 1048576u64.into()),
        ].into(),
        content_handle: None,
    };

    // 添加所有节点
    graph.add_node(root.clone()).await?;
    graph.add_node(home.clone()).await?;
    graph.add_node(user.clone()).await?;
    graph.add_node(documents.clone()).await?;
    graph.add_node(file1.clone()).await?;
    graph.add_node(file2.clone()).await?;
    graph.add_node(downloads.clone()).await?;
    graph.add_node(file3.clone()).await?;

    // 创建边（目录层次结构）
    let edges = vec![
        (root.id, home.id, EdgeType::Contains),
        (home.id, user.id, EdgeType::Contains),
        (user.id, documents.id, EdgeType::Contains),
        (documents.id, file1.id, EdgeType::Contains),
        (documents.id, file2.id, EdgeType::Contains),
        (user.id, downloads.id, EdgeType::Contains),
        (downloads.id, file3.id, EdgeType::Contains),
        // 添加一些交叉连接
        (file1.id, file2.id, EdgeType::References),
    ];

    for (source, target, edge_type) in edges {
        let edge = Edge {
            id: Uuid::new_v4(),
            source,
            target,
            edge_type,
            weight: None,
            properties: HashMap::new(),
        };
        graph.add_edge(edge).await?;
    }

    Ok(graph)
}
