// CLI 重定向支持
// 实现输出重定向 (>, >>) 和输入重定向 (<)

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

/// 重定向类型
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectionType {
    /// 覆盖输出 (>)
    Output,
    /// 追加输出 (>>)
    Append,
    /// 输入重定向 (<)
    Input,
}

/// 重定向配置
#[derive(Debug, Clone)]
pub struct Redirection {
    pub redirect_type: RedirectionType,
    pub target: String,
}

/// 解析命令行中的重定向符号
/// 返回 (命令部分, 重定向配置)
pub fn parse_redirection(input: &str) -> (String, Option<Redirection>) {
    let input = input.trim();

    // 检查输入重定向 (<)
    if let Some(pos) = input.find(" < ") {
        let (before, after) = input.split_at(pos);
        let target = after[3..].trim().to_string();
        if !target.is_empty() {
            return (
                before.trim().to_string(),
                Some(Redirection {
                    redirect_type: RedirectionType::Input,
                    target,
                }),
            );
        }
    }

    // 检查追加重定向 (>>) - 必须在覆盖重定向 (>) 之前检查
    if let Some(pos) = input.rfind(" >> ") {
        let (before, after) = input.split_at(pos);
        let target = after[4..].trim().to_string();
        if !target.is_empty() {
            return (
                before.trim().to_string(),
                Some(Redirection {
                    redirect_type: RedirectionType::Append,
                    target,
                }),
            );
        }
    }

    // 检查覆盖重定向 (>)
    if let Some(pos) = input.rfind(" > ") {
        let (before, after) = input.split_at(pos);
        let target = after[3..].trim().to_string();
        if !target.is_empty() {
            return (
                before.trim().to_string(),
                Some(Redirection {
                    redirect_type: RedirectionType::Output,
                    target,
                }),
            );
        }
    }

    (input.to_string(), None)
}

/// 执行输入重定向：从文件读取内容
pub fn read_from_file(path: &str) -> io::Result<String> {
    let path = Path::new(path);

    // 检查文件是否存在
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        ));
    }

    // 检查是否是目录
    if path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Is a directory: {}", path.display()),
        ));
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// 执行输出重定向：将内容写入文件
pub fn write_to_file(path: &str, content: &str, append: bool) -> io::Result<()> {
    let path = Path::new(path);
    let parent = path.parent();

    // 确保父目录存在
    if let Some(parent_dir) = parent {
        if !parent_dir.as_os_str().is_empty() {
            std::fs::create_dir_all(parent_dir)?;
        }
    }

    let file = if append {
        // 追加模式：文件不存在则创建
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
    } else {
        // 覆盖模式：创建新文件（如果存在则截断）
        File::create(path)?
    };

    // 使用 BufWriter 提高写入性能
    let mut writer = io::BufWriter::new(file);
    writer.write_all(content.as_bytes())?;
    writer.flush()?;

    Ok(())
}

/// 执行输出重定向到文件
pub fn redirect_output(path: &str, content: &str, redirect_type: &RedirectionType) -> io::Result<()> {
    match redirect_type {
        RedirectionType::Output | RedirectionType::Append => {
            let append = *redirect_type == RedirectionType::Append;
            write_to_file(path, content, append)
        }
        RedirectionType::Input => {
            // 输入重定向不应该调用此函数
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid redirection type for output",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_output_redirect() {
        let (cmd, redir) = parse_redirection("cat /mem/file.txt > output.txt");
        assert_eq!(cmd, "cat /mem/file.txt");
        assert!(redir.is_some());
        let r = redir.unwrap();
        assert_eq!(r.redirect_type, RedirectionType::Output);
        assert_eq!(r.target, "output.txt");
    }

    #[test]
    fn test_parse_append_redirect() {
        let (cmd, redir) = parse_redirection("cat /mem/file.txt >> output.txt");
        assert_eq!(cmd, "cat /mem/file.txt");
        assert!(redir.is_some());
        let r = redir.unwrap();
        assert_eq!(r.redirect_type, RedirectionType::Append);
        assert_eq!(r.target, "output.txt");
    }

    #[test]
    fn test_parse_input_redirect() {
        let (cmd, redir) = parse_redirection("write /mem/file.txt < input.txt");
        assert_eq!(cmd, "write /mem/file.txt");
        assert!(redir.is_some());
        let r = redir.unwrap();
        assert_eq!(r.redirect_type, RedirectionType::Input);
        assert_eq!(r.target, "input.txt");
    }

    #[test]
    fn test_no_redirect() {
        let (cmd, redir) = parse_redirection("ls /mem");
        assert_eq!(cmd, "ls /mem");
        assert!(redir.is_none());
    }

    #[test]
    fn test_redirect_with_path() {
        let (cmd, redir) = parse_redirection("cat /mem/data/file.txt > /tmp/output.txt");
        assert_eq!(cmd, "cat /mem/data/file.txt");
        assert!(redir.is_some());
        assert_eq!(redir.unwrap().target, "/tmp/output.txt");
    }

    #[test]
    fn test_append_before_output() {
        // 确保 >> 不会被误解析为 >
        let (cmd, redir) = parse_redirection("echo hello >> log.txt");
        assert_eq!(cmd, "echo hello");
        assert_eq!(redir.unwrap().redirect_type, RedirectionType::Append);
    }

    #[test]
    fn test_write_output() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("evif_redirect_test.txt");
        let path_str = path.to_str().unwrap();

        let content = "Hello, World!\nLine 2\n";
        write_to_file(path_str, content, false).unwrap();

        // 读取验证
        let read_content = fs::read_to_string(path_str).unwrap();
        assert_eq!(read_content, content);

        // 清理
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_append_output() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("evif_append_test.txt");
        let path_str = path.to_str().unwrap();

        // 第一次写入
        write_to_file(path_str, "First line\n", false).unwrap();

        // 追加
        write_to_file(path_str, "Second line\n", true).unwrap();

        // 读取验证
        let content = fs::read_to_string(path_str).unwrap();
        assert_eq!(content, "First line\nSecond line\n");

        // 清理
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_overwrite_not_append() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("evif_overwrite_test.txt");
        let path_str = path.to_str().unwrap();

        // 第一次写入
        write_to_file(path_str, "Original content\n", false).unwrap();

        // 覆盖（非追加）
        write_to_file(path_str, "New content\n", false).unwrap();

        // 读取验证 - 应该只有新内容
        let content = fs::read_to_string(path_str).unwrap();
        assert_eq!(content, "New content\n");

        // 清理
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_read_input() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("evif_input_test.txt");
        let path_str = path.to_str().unwrap();

        // 创建测试文件
        fs::write(path_str, "Test input content\n").unwrap();

        // 读取验证
        let content = read_from_file(path_str).unwrap();
        assert_eq!(content, "Test input content\n");

        // 清理
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_from_file("/nonexistent/path/to/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_parent_directories() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("evif_nested/level1/level2/test.txt");
        let path_str = path.to_str().unwrap();

        // 写入应该自动创建父目录
        write_to_file(path_str, "Nested content\n", false).unwrap();

        // 读取验证
        let content = fs::read_to_string(path_str).unwrap();
        assert_eq!(content, "Nested content\n");

        // 清理
        fs::remove_dir_all(temp_dir.join("evif_nested")).ok();
    }

    #[test]
    fn test_redirect_type_equality() {
        assert_eq!(RedirectionType::Output, RedirectionType::Output);
        assert_eq!(RedirectionType::Append, RedirectionType::Append);
        assert_eq!(RedirectionType::Input, RedirectionType::Input);
        assert_ne!(RedirectionType::Output, RedirectionType::Append);
        assert_ne!(RedirectionType::Output, RedirectionType::Input);
    }
}
