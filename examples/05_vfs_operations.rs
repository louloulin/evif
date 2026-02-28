//! EVIF 虚拟文件系统操作示例
//!
//! 演示VFS层的基本文件操作：创建、读取、写入、删除

use evif_vfs::{Vfs, FileSystem, OpenFlags, FileAttributes, FileType as VfsFileType};
use evif_graph::{NodeType, Metadata};
use std::path::Path;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF 虚拟文件系统操作示例 ===\n");

    // 1. 创建VFS实例
    println!("1. 创建VFS实例...");
    let vfs = Vfs::new();
    println!("✓ VFS创建成功\n");

    // 2. 创建目录
    println!("2. 创建目录结构...");
    vfs.mkdir(Path::new("/documents"), 0o755).await?;
    vfs.mkdir(Path::new("/documents/work"), 0o755).await?;
    vfs.mkdir(Path::new("/documents/personal"), 0o755).await?;
    println!("✓ 创建目录:");
    println!("  - /documents");
    println!("  - /documents/work");
    println!("  - /documents/personal\n");

    // 3. 创建并写入文件
    println!("3. 创建并写入文件...");
    let file_handle = vfs.open(
        Path::new("/documents/work/notes.txt"),
        OpenFlags::WRITE | OpenFlags::CREATE
    ).await?;

    let content = b"EVIF文件系统示例\n这是一个测试文件";
    vfs.write(file_handle, 0, content).await?;
    vfs.close(file_handle).await?;
    println!("✓ 创建文件: /documents/work/notes.txt");
    println!("  内容长度: {} bytes\n", content.len());

    // 4. 读取文件
    println!("4. 读取文件...");
    let read_handle = vfs.open(
        Path::new("/documents/work/notes.txt"),
        OpenFlags::READ
    ).await?;

    let mut read_buffer = vec![0u8; 1024];
    let bytes_read = vfs.read(read_handle, 0, &mut read_buffer).await?;
    read_buffer.truncate(bytes_read);
    vfs.close(read_handle).await?;

    println!("✓ 读取文件成功:");
    println!("  读取字节数: {}", bytes_read);
    println!("  内容:\n{}", String::from_utf8_lossy(&read_buffer));
    println!();

    // 5. 获取文件属性
    println!("5. 获取文件属性...");
    let attrs = vfs.getattr(Path::new("/documents/work/notes.txt")).await?;
    println!("✓ 文件属性:");
    println!("  - 大小: {} bytes", attrs.size);
    println!("  - 类型: {:?}", attrs.file_type);
    println!("  - 权限: 0o{:o}", attrs.mode);
    println!("  - 创建时间: {:?}", attrs.created);
    println!("  - 修改时间: {:?}", attrs.modified);
    println!();

    // 6. 列出目录内容
    println!("6. 列出目录内容...");
    let entries = vfs.readdir(Path::new("/documents")).await?;
    println!("✓ /documents 包含 {} 项:", entries.len());
    for entry in &entries {
        println!("  - {} ({})", entry.name,
                 if entry.file_type == VfsFileType::Directory { "DIR" } else { "FILE" });
    }
    println!();

    // 7. 追加写入
    println!("7. 追加写入文件...");
    let append_handle = vfs.open(
        Path::new("/documents/work/notes.txt"),
        OpenFlags::WRITE | OpenFlags::APPEND
    ).await?;

    let append_content = b"\n追加的内容";
    vfs.write(append_handle, 0, append_content).await?;
    vfs.close(append_handle).await?;
    println!("✓ 追加 {} bytes\n", append_content.len());

    // 8. 再次读取验证
    println!("8. 验证追加结果...");
    let verify_handle = vfs.open(
        Path::new("/documents/work/notes.txt"),
        OpenFlags::READ
    ).await?;

    let mut verify_buffer = vec![0u8; 2048];
    let verify_bytes = vfs.read(verify_handle, 0, &mut verify_buffer).await?;
    verify_buffer.truncate(verify_bytes);
    vfs.close(verify_handle).await?;

    println!("✓ 文件新大小: {} bytes", verify_bytes);
    println!("  总内容:\n{}", String::from_utf8_lossy(&verify_buffer));
    println!();

    // 9. 文件seek操作
    println!("9. 测试文件seek操作...");
    let seek_handle = vfs.open(
        Path::new("/documents/work/notes.txt"),
        OpenFlags::READ
    ).await?;

    // Seek到文件中间
    let seek_pos = 10;
    vfs.seek(seek_handle, seek_pos as u64).await?;
    println!("✓ Seek到位置: {}", seek_pos);

    let mut seek_buffer = vec![0u8; 20];
    let seek_bytes = vfs.read(seek_handle, seek_pos as u64, &mut seek_buffer).await?;
    seek_buffer.truncate(seek_bytes);
    vfs.close(seek_handle).await?;

    println!("  从位置{}读取: {}", seek_pos, String::from_utf8_lossy(&seek_buffer));
    println!();

    // 10. 删除文件
    println!("10. 删除文件...");
    let file_to_delete = Path::new("/documents/work/notes.txt");
    vfs.unlink(file_to_delete).await?;
    println!("✓ 文件已删除: /documents/work/notes.txt\n");

    // 11. 验证删除
    println!("11. 验证文件已删除...");
    match vfs.open(Path::new("/documents/work/notes.txt"), OpenFlags::READ).await {
        Ok(_) => println!("✗ 文件仍然存在（意外）\n"),
        Err(_) => println!("✓ 文件确实已删除\n"),
    }

    // 12. 删除目录
    println!("12. 删除空目录...");
    vfs.rmdir(Path::new("/documents/work")).await?;
    println!("✓ 目录已删除: /documents/work\n");

    // 13. VFS统计信息
    println!("13. VFS统计信息...");
    let stats = vfs.stats().await?;
    println!("✓ VFS统计:");
    println!("  - 总文件数: {}", stats.total_files);
    println!("  - 总目录数: {}", stats.total_dirs);
    println!("  - 总大小: {} bytes", stats.total_size);
    println!();

    println!("=== 示例完成 ===");
    Ok(())
}
