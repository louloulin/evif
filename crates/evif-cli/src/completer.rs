// 命令自动补全和路径补全

use reedline::{Completer, Suggestion};

pub struct EvifCompleter {
    commands: Vec<String>,
}

impl EvifCompleter {
    pub fn new(_server: String) -> Self {
        let commands = vec![
            // 文件操作
            "ls".to_string(),
            "cat".to_string(),
            "write".to_string(),
            "mkdir".to_string(),
            "rm".to_string(),
            "mv".to_string(),
            "cp".to_string(),
            "stat".to_string(),
            "touch".to_string(),
            // 高级操作
            "head".to_string(),
            "tail".to_string(),
            "tree".to_string(),
            "find".to_string(),
            "grep".to_string(),
            "digest".to_string(),
            "diff".to_string(),
            "du".to_string(),
            "file".to_string(),
            // 文本处理
            "sort".to_string(),
            "uniq".to_string(),
            "wc".to_string(),
            "cut".to_string(),
            "tr".to_string(),
            "rev".to_string(),
            "tac".to_string(),
            "base".to_string(),
            "truncate".to_string(),
            "split".to_string(),
            // 系统操作
            "echo".to_string(),
            "cd".to_string(),
            "pwd".to_string(),
            "date".to_string(),
            "sleep".to_string(),
            "env".to_string(),
            "export".to_string(),
            "unset".to_string(),
            "true".to_string(),
            "false".to_string(),
            // 路径操作
            "basename".to_string(),
            "dirname".to_string(),
            "realpath".to_string(),
            "readlink".to_string(),
            "ln".to_string(),
            // 查找命令
            "locate".to_string(),
            "which".to_string(),
            "type".to_string(),
            // 插件操作
            "mount".to_string(),
            "unmount".to_string(),
            "mounts".to_string(),
            // 服务器操作
            "health".to_string(),
            "stats".to_string(),
            // 文件传输
            "upload".to_string(),
            "download".to_string(),
            // 脚本操作
            "source".to_string(),
            ".".to_string(),
            // 其他
            "clear".to_string(),
            "help".to_string(),
            "exit".to_string(),
            "quit".to_string(),
            "query".to_string(),
            "get".to_string(),
            "create".to_string(),
            "delete".to_string(),
            "repl".to_string(),
            "list-mounts".to_string(),
            "mount-plugin".to_string(),
            "unmount-plugin".to_string(),
            "umount".to_string(),
        ];

        Self { commands }
    }

    /// 根据前缀过滤命令
    fn complete_commands(&self, prefix: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// 获取常见挂载点列表
    fn get_common_mounts(&self) -> Vec<String> {
        vec![
            "/mem".to_string(),
            "/local".to_string(),
            "/hello".to_string(),
            "/s3".to_string(),
            "/http".to_string(),
        ]
    }

    /// 检查路径是否是EVIF路径（以/开头）
    fn is_evif_path(path: &str) -> bool {
        path.starts_with('/')
    }

    /// 补全EVIF路径
    fn complete_evif_path(&self, prefix: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // 如果前缀为空或只是根路径，返回常见挂载点
        if prefix.is_empty() || prefix == "/" {
            return self.get_common_mounts();
        }

        // 尝试匹配路径前缀
        for mount in self.get_common_mounts() {
            if mount.starts_with(prefix) {
                suggestions.push(mount);
            } else if prefix.starts_with(&mount) {
                // 如果前缀以某个挂载点开头，返回该路径本身（可能的文件或子目录）
                suggestions.push(prefix.to_string());
            }
        }

        suggestions
    }
}

impl Completer for EvifCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // 获取当前单词
        let line_before_cursor = &line[..pos];
        let words: Vec<&str> = line_before_cursor.split_whitespace().collect();

        if words.is_empty() {
            // 空行，补全所有命令
            for cmd in self.commands.iter() {
                suggestions.push(Suggestion {
                    span: reedline::Span::new(0, pos),
                    value: cmd.clone(),
                    description: None,
                    append_whitespace: false,
                    extra: None,
                });
            }
        } else if words.len() == 1 {
            // 正在输入第一个单词（命令）
            let current_cmd = words[0];
            for cmd in self.complete_commands(current_cmd) {
                suggestions.push(Suggestion {
                    span: reedline::Span::new(0, pos),
                    value: cmd,
                    description: None,
                    append_whitespace: true,
                    extra: None,
                });
            }
        } else {
            // 补全路径或其他参数
            let last_word_start = if let Some(idx) = line_before_cursor.rfind(' ') {
                idx + 1
            } else {
                0
            };
            let last_word = &line_before_cursor[last_word_start..];

            // 实现路径补全
            if Self::is_evif_path(last_word) {
                // EVIF路径补全
                for path in self.complete_evif_path(last_word) {
                    suggestions.push(Suggestion {
                        span: reedline::Span::new(last_word_start, pos),
                        value: path,
                        description: None,
                        append_whitespace: false,
                        extra: None,
                    });
                }
            }
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_commands() {
        let completer = EvifCompleter::new("localhost:8080".to_string());

        let results = completer.complete_commands("l");
        assert!(results.contains(&"ls".to_string()));
        assert!(results.contains(&"locate".to_string()));
        assert!(results.contains(&"ln".to_string()));
        assert!(results.contains(&"list-mounts".to_string()));
    }

    #[test]
    fn test_completer() {
        let mut completer = EvifCompleter::new("localhost:8080".to_string());

        let suggestions = completer.complete("l", 1);
        assert!(!suggestions.is_empty());
    }
}
