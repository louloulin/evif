//! EVIF 认证授权示例
//!
//! 演示基于能力的安全模型和权限控制

use evif_auth::{
    AuthManager, Capability, Permissions, Principal, AuthPolicy,
};
use evif_graph::{NodeId, NodeType};
use std::time::SystemTime;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF 认证授权系统示例 ===\n");

    // 1. 创建认证管理器
    println!("1. 创建认证管理器...");
    let auth = AuthManager::new(AuthPolicy::Open);
    println!("✓ 认证管理器创建成功 (开放策略)\n");

    // 2. 创建测试主体
    println!("2. 创建测试主体...");
    let user_alice = Principal::User(Uuid::new_v4());
    let user_bob = Principal::User(Uuid::new_v4());
    println!("✓ 创建用户: Alice, Bob\n");

    // 3. 创建资源节点
    println!("3. 创建资源节点...");
    let file_node = NodeId::new();
    let dir_node = NodeId::new();
    println!("✓ 创建资源: file_node, dir_node\n");

    // 4. 授予Alice权限
    println!("4. 授予Alice对file_node的读写权限...");
    let alice_cap = Capability {
        id: Uuid::new_v4(),
        holder: user_alice.clone(),
        node: file_node,
        permissions: Permissions {
            read: true,
            write: true,
            execute: false,
            admin: false,
        },
        expires: None,
    };

    let cap_id = auth.grant(alice_cap).await?;
    println!("✓ 能力已授予: {}\n", cap_id);

    // 5. 验证Alice的读权限
    println!("5. 验证Alice的读权限...");
    let can_read = auth.check(&user_alice, &file_node, evif_auth::Permission::Read).await?;
    println!("✓ Alice可以读取file_node: {}\n", can_read);

    // 6. 验证Alice的执行权限
    println!("6. 验证Alice的执行权限...");
    let can_execute = auth.check(&user_alice, &file_node, evif_auth::Permission::Execute).await?;
    println!("✓ Alice可以执行file_node: {}\n", can_execute);

    // 7. Bob尝试访问（无权限）
    println!("7. Bob尝试访问file_node...");
    let bob_can_read = auth.check(&user_bob, &file_node, evif_auth::Permission::Read).await?;
    println!("✓ Bob可以读取file_node: {}\n", bob_can_read);

    // 8. 授予Bob只读权限
    println!("8. 授予Bob只读权限...");
    let bob_cap = Capability {
        id: Uuid::new_v4(),
        holder: user_bob.clone(),
        node: file_node,
        permissions: Permissions {
            read: true,
            write: false,
            execute: false,
            admin: false,
        },
        expires: None,
    };
    auth.grant(bob_cap).await?;
    println!("✓ Bob已被授予只读权限\n");

    // 9. 再次验证Bob的读权限
    println!("9. 再次验证Bob的读权限...");
    let bob_can_read_now = auth.check(&user_bob, &file_node, evif_auth::Permission::Read).await?;
    println!("✓ Bob现在可以读取file_node: {}\n", bob_can_read_now);

    // 10. 撤销Alice的能力
    println!("10. 撤销Alice的权限...");
    auth.revoke(&cap_id).await?;
    println!("✓ Alice的权限已被撤销\n");

    // 11. 验证撤销后的权限
    println!("11. 验证Alice撤销后的读权限...");
    let alice_can_read_after = auth.check(&user_alice, &file_node, evif_auth::Permission::Read).await?;
    println!("✓ Alice可以读取file_node: {}\n", alice_can_read_after);

    // 12. 创建带过期时间的能力
    println!("12. 创建带过期时间的能力（1秒后过期）...");
    let temp_cap = Capability {
        id: Uuid::new_v4(),
        holder: user_bob.clone(),
        node: dir_node,
        permissions: Permissions {
            read: true,
            write: true,
            execute: true,
            admin: false,
        },
        expires: Some(SystemTime::now() + std::time::Duration::from_secs(1)),
    };
    let temp_cap_id = auth.grant(temp_cap).await?;
    println!("✓ 临时能力已授予: {}\n", temp_cap_id);

    // 13. 立即验证临时能力
    println!("13. 立即验证临时能力...");
    let can_access_now = auth.check(&user_bob, &dir_node, evif_auth::Permission::Write).await?;
    println!("✓ Bob现在可以访问dir_node: {}\n", can_access_now);

    // 14. 等待过期后验证
    println!("14. 等待能力过期（2秒）...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let can_access_after = auth.check(&user_bob, &dir_node, evif_auth::Permission::Write).await?;
    println!("✓ 过期后Bob可以访问dir_node: {}\n", can_access_after);

    println!("=== 示例完成 ===");
    Ok(())
}
