// REPL 实现

use crate::commands::EvifCommand;
use crate::completer::EvifCompleter;
use anyhow::Result;
use reedline::DefaultPromptSegment;
use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct Repl {
    editor: Reedline,
    prompt: DefaultPrompt,
    command: EvifCommand,
}

impl Repl {
    pub fn new(server: String, verbose: bool) -> Self {
        // 配置历史记录文件
        let history_path = Self::history_file_path();
        let history = Box::new(
            FileBackedHistory::with_file(1000, history_path)
                .expect("Failed to create history file"),
        );

        // 创建自动完成器
        let completer = Box::new(EvifCompleter::new(server.clone()));

        // 创建 Reedline 编辑器，启用历史和自动完成
        let editor = Reedline::create()
            .with_history(history)
            .with_completer(completer);

        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic("evif".to_string()),
            DefaultPromptSegment::Basic(format!("{}>", server)),
        );

        if verbose {
            println!("Verbose mode enabled");
        }

        let command = EvifCommand::new(server.clone(), verbose);
        Self {
            editor,
            prompt,
            command,
        }
    }

    /// 获取历史文件路径
    fn history_file_path() -> PathBuf {
        // 优先使用 XDG_DATA_HOME，然后是 ~/.local/share
        let base_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .ok()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));

        base_dir.join("evif").join("history.txt")
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("EVIF Interactive REPL v1.8");
        println!("Type 'help' for available commands, 'exit' or 'quit' to leave");
        println!();

        loop {
            let sig = self.editor.read_line(&self.prompt);

            match sig {
                Ok(Signal::Success(line)) => {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    // 支持管道和重定向
                    if line.contains('|') || line.contains('>') {
                        if let Err(e) = self.handle_shell_syntax(line).await {
                            eprintln!("Error: {}", e);
                        }
                        continue;
                    }

                    if self.handle_command(line).await? {
                        break;
                    }
                }
                Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                    println!("\nGoodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                }
            }
        }

        Ok(())
    }

    async fn handle_shell_syntax(&mut self, line: &str) -> Result<()> {
        if line.contains('|') {
            let output = self.execute_pipeline_capture(line).await?;
            if !output.is_empty() {
                print!("{}", output);
            }
            return Ok(());
        }

        self.handle_redirection(line).await
    }

    async fn builtin_output(&self, line: &str) -> Result<Option<String>> {
        let expanded_line = self.command.expand_variables(line);
        let parts: Vec<&str> = expanded_line.split_whitespace().collect();
        let cmd = parts.first().copied().unwrap_or("");

        let output = match cmd {
            "echo" => Some(self.command.echo_output(parts.get(1..).unwrap_or(&[]).join(" "))),
            "pwd" => Some(self.command.pwd_output()),
            "cat" => {
                let path = parts
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("Usage: cat <file>"))?;
                Some(self.command.cat_output((*path).to_string()).await?)
            }
            "ls" => {
                let path = parts.get(1).map(|s| s.to_string()).or_else(|| Some("/".to_string()));
                Some(self.command.ls_output(path, false).await?)
            }
            _ => None,
        };

        Ok(output)
    }

    fn spawn_external_pipeline(
        &self,
        commands: &[&str],
        mut initial_input: Option<Vec<u8>>,
    ) -> Result<String> {
        use std::process::{Command, Stdio};

        let mut prev_stdout: Option<std::process::ChildStdout> = None;
        let mut children = Vec::new();

        for command_str in commands {
            let command_str = command_str.trim();
            if command_str.is_empty() {
                continue;
            }

            let parts: Vec<&str> = command_str.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let cmd = parts[0];
            let args = &parts[1..];

            let mut process = Command::new(cmd);
            process.args(args);

            let needs_piped_input = prev_stdout.is_some() || initial_input.is_some();
            if let Some(stdout) = prev_stdout.take() {
                process.stdin(Stdio::from(stdout));
            } else if needs_piped_input {
                process.stdin(Stdio::piped());
            } else {
                process.stdin(Stdio::inherit());
            }

            process.stdout(Stdio::piped());
            process.stderr(Stdio::inherit());

            let mut child = process.spawn()?;

            if let Some(input) = initial_input.take() {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(&input)?;
                }
            }

            prev_stdout = child.stdout.take();
            children.push(child);
        }

        let mut output = String::new();
        if let Some(mut stdout) = prev_stdout {
            let mut buffer = Vec::new();
            stdout.read_to_end(&mut buffer)?;
            output = String::from_utf8_lossy(&buffer).to_string();
        }

        for mut child in children {
            let _ = child.wait();
        }

        Ok(output)
    }

    async fn execute_pipeline_capture(&mut self, line: &str) -> Result<String> {
        let commands: Vec<&str> = line.split('|').collect();

        if commands.len() == 1 {
            if let Some(output) = self.builtin_output(commands[0].trim()).await? {
                return Ok(output);
            }
            return self.spawn_external_pipeline(&commands, None);
        }

        if let Some(output) = self.builtin_output(commands[0].trim()).await? {
            for stage in commands.iter().skip(1) {
                if self.builtin_output(stage.trim()).await?.is_some() {
                    return Err(anyhow::anyhow!(
                        "Built-in commands are only supported as the first stage of a pipeline in this phase"
                    ));
                }
            }

            return self.spawn_external_pipeline(&commands[1..], Some(output.into_bytes()));
        }

        self.spawn_external_pipeline(&commands, None)
    }

    async fn handle_redirection(&mut self, line: &str) -> Result<()> {
        let (command_part, target, append) = if let Some((left, right)) = line.split_once(">>") {
            (left.trim(), right.trim(), true)
        } else if let Some((left, right)) = line.split_once('>') {
            (left.trim(), right.trim(), false)
        } else {
            return Ok(());
        };

        let output = if let Some(output) = self.builtin_output(command_part).await? {
            output
        } else {
            self.spawn_external_pipeline(&[command_part], None)?
        };

        let target_path = std::path::Path::new(target);
        let treat_as_local = !target.starts_with('/')
            || target_path
                .parent()
                .map(|parent| parent.exists())
                .unwrap_or(false);

        if treat_as_local {
            if append {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(target)?;
                file.write_all(output.as_bytes())?;
            } else {
                std::fs::write(target, output)?;
            }
        } else {
            self.command
                .write(target.to_string(), output.trim_end_matches('\n').to_string(), append)
                .await?;
        }

        Ok(())
    }

    async fn handle_command(&mut self, line: &str) -> Result<bool> {
        // 展开变量
        let expanded_line = self.command.expand_variables(line);
        let parts: Vec<&str> = expanded_line.split_whitespace().collect();
        let cmd = parts.first().copied().unwrap_or("");

        match cmd {
            "exit" | "quit" => {
                return Ok(true);
            }
            "help" => {
                self.print_help();
            }
            "ls" => {
                let path = parts
                    .get(1)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "/".to_string());
                if let Err(e) = self.command.ls(Some(path), false, false).await {
                    eprintln!("Error: {}", e);
                }
            }
            "cat" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.cat(path.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: cat <file>");
                }
            }
            "write" => {
                if let Some(path) = parts.get(1) {
                    let content = parts[2..].join(" ");
                    if let Err(e) = self.command.write(path.to_string(), content, false).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: write <file> <content>");
                }
            }
            "mkdir" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.mkdir(path.to_string(), false).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: mkdir <path>");
                }
            }
            "rm" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.rm(path.to_string(), false).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: rm <path>");
                }
            }
            "mv" => {
                if let (Some(src), Some(dst)) = (parts.get(1), parts.get(2)) {
                    if let Err(e) = self.command.mv(src.to_string(), dst.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: mv <src> <dst>");
                }
            }
            "cp" => {
                if let (Some(src), Some(dst)) = (parts.get(1), parts.get(2)) {
                    if let Err(e) = self.command.cp(src.to_string(), dst.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: cp <src> <dst>");
                }
            }
            "stat" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.stat(path.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: stat <path>");
                }
            }
            "touch" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.touch(path.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: touch <file>");
                }
            }
            "head" => {
                if let Some(path) = parts.get(1) {
                    let lines = parts
                        .get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(10);
                    if let Err(e) = self.command.head(path.to_string(), lines).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: head <file> [lines]");
                }
            }
            "tail" => {
                if let Some(path) = parts.get(1) {
                    let lines = parts
                        .get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(10);
                    if let Err(e) = self.command.tail(path.to_string(), lines).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: tail <file> [lines]");
                }
            }
            "tree" => {
                let path = parts
                    .get(1)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "/".to_string());
                let depth = parts
                    .get(2)
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(3);
                if let Err(e) = self.command.tree(path, depth, depth).await {
                    eprintln!("Error: {}", e);
                }
            }
            "find" => {
                if let Some(path) = parts.get(1) {
                    let pattern = parts.get(2).copied();
                    let type_ = parts.get(3).copied();
                    if let Err(e) = self.command.find(path.to_string(), pattern, type_).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: find <path> [pattern] [type]");
                }
            }
            "grep" => {
                if let (Some(path), Some(pattern)) = (parts.get(1), parts.get(2)) {
                    let recursive = parts
                        .get(3)
                        .map(|s| *s == "-r" || *s == "--recursive")
                        .unwrap_or(false);
                    if let Err(e) = self
                        .command
                        .grep(path.to_string(), pattern.to_string(), recursive)
                        .await
                    {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: grep <path> <pattern> [-r|--recursive]");
                }
            }
            "digest" | "checksum" => {
                if let Some(path) = parts.get(1) {
                    let algorithm = parts
                        .get(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "sha256".to_string());
                    if let Err(e) = self.command.checksum(path.to_string(), algorithm).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: digest <path> [algorithm]  (algorithm: sha256, sha512)");
                }
            }
            "mount" => {
                if let Some(plugin) = parts.get(1) {
                    let path = parts
                        .get(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("/{}", plugin));
                    if let Err(e) = self.command.mount(plugin.to_string(), path, None).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: mount <plugin> [path]");
                }
            }
            "unmount" => {
                if let Some(path) = parts.get(1) {
                    if let Err(e) = self.command.unmount(path.to_string()).await {
                        eprintln!("Error: {}", e);
                    }
                } else {
                    println!("Usage: unmount <path>");
                }
            }
            "mounts" => {
                if let Err(e) = self.command.mounts().await {
                    eprintln!("Error: {}", e);
                }
            }
            "health" => {
                if let Err(e) = self.command.health().await {
                    eprintln!("Error: {}", e);
                }
            }
            "stats" => {
                if let Err(e) = self.command.stats().await {
                    eprintln!("Error: {}", e);
                }
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
            }
            "source" | "." => {
                if let Some(script_path) = parts.get(1) {
                    // 使用 ScriptExecutor 执行脚本
                    let script_path_expanded = self.command.expand_variables(script_path);
                    match crate::script::ScriptExecutor::execute_script_with_client(
                        &script_path_expanded,
                        &self.command,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(e) => eprintln!("Error executing script: {}", e),
                    }
                } else {
                    println!("Usage: source <script.as>");
                }
            }
            "export" => {
                if parts.len() >= 2 {
                    // 重新组合参数以支持 export VAR=value 语法
                    let arg = parts[1..].join(" ");
                    match self.command.export(arg).await {
                        Ok(_) => println!("Variable exported"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    println!("Usage: export VAR=value");
                }
            }
            "unset" => {
                if let Some(var_name) = parts.get(1) {
                    match self.command.unset(var_name.to_string()).await {
                        Ok(_) => println!("Variable unset"),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    println!("Usage: unset VAR");
                }
            }
            "set" => {
                // 兼容 AGFS 的 set 命令语法
                if parts.len() >= 3 {
                    let var_name = parts[1].to_string();
                    let var_value = parts[2..].join(" ");
                    self.command.set_variable(var_name, var_value);
                    println!("Variable set");
                } else if parts.len() == 2 {
                    // 如果是 VAR=value 格式
                    if let Some((name, value)) = parts[1].split_once('=') {
                        self.command
                            .set_variable(name.to_string(), value.to_string());
                        println!("Variable set");
                    } else {
                        println!("Usage: set VAR value or set VAR=value");
                    }
                } else {
                    println!("Usage: set VAR value");
                }
            }
            "echo" => {
                // 支持变量展开的 echo
                let text = parts
                    .get(1)
                    .map(|_s| parts[1..].join(" "))
                    .unwrap_or_default();
                // 使用 expand_variables 处理变量引用
                let expanded = self.command.expand_variables(&text);
                println!("{}", expanded);
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    cmd
                );
            }
        }

        Ok(false)
    }

    fn print_help(&self) {
        println!("Available commands:");
        println!();
        println!("File Operations:");
        println!("  ls [path]          - List directory contents");
        println!("  cat <file>         - Display file contents");
        println!("  write <file> <data> - Write data to file");
        println!("  mkdir <path>       - Create directory");
        println!("  rm <path>          - Remove file or directory");
        println!("  mv <src> <dst>     - Move/rename file");
        println!("  cp <src> <dst>     - Copy file");
        println!("  stat <path>        - Display file status");
        println!("  touch <file>       - Create empty file");
        println!();
        println!("Advanced Operations:");
        println!("  head <file> [n]    - Display first n lines (default: 10)");
        println!("  tail <file> [n]    - Display last n lines (default: 10)");
        println!("  tree [path] [depth] - Display directory tree (default depth: 3)");
        println!("  find <path> <pattern> - Search for files matching pattern");
        println!("  grep <path> <pattern> [-r] - Regex search in path");
        println!("  digest <path> [algo] - File checksum (sha256/sha512)");
        println!();
        println!("Variable Support:");
        println!("  export VAR=value   - Set and export a variable");
        println!("  unset VAR          - Remove a variable");
        println!("  set VAR value      - Set a variable (alternative syntax)");
        println!("  env                - List all variables");
        println!("  echo $VAR          - Print variable value (with expansion)");
        println!("  Note: Variables support $VAR and ${{VAR}} syntax");
        println!();
        println!("Script Execution:");
        println!("  source <script.as> - Execute AGFS script file");
        println!("  . <script.as>      - Execute AGFS script file (shorthand)");
        println!();
        println!("Plugin Management:");
        println!("  mount <plugin> <path> - Mount plugin at path");
        println!("  unmount <path>     - Unmount plugin");
        println!("  mounts             - List mounted plugins");
        println!();
        println!("Server Operations:");
        println!("  health             - Check server health");
        println!("  stats              - Show statistics");
        println!();
        println!("Other:");
        println!("  clear              - Clear screen");
        println!("  help               - Show this help message");
        println!("  exit/quit          - Exit REPL");
        println!();
        println!("Pipeline Support (Experimental):");
        println!("  cmd1 | cmd2        - Pipe output of cmd1 to cmd2");
        println!("  cmd > file         - Redirect output to file");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_repl_creation() {
        let _repl = Repl::new("localhost:50051".to_string(), false);
        // REPL creation successful
        assert!(Repl::history_file_path().to_string_lossy().contains('/'));
    }

    #[tokio::test]
    async fn test_builtin_echo_can_feed_external_pipeline() {
        let mut repl = Repl::new("localhost:50051".to_string(), false);
        let output = repl
            .execute_pipeline_capture("echo hello-from-evif | grep hello")
            .await
            .expect("pipeline");

        assert_eq!(output.trim(), "hello-from-evif");
    }

    #[tokio::test]
    async fn test_builtin_echo_can_redirect_to_local_file() {
        let mut repl = Repl::new("localhost:50051".to_string(), false);
        let temp_dir = tempdir().expect("tempdir");
        let target = temp_dir.path().join("redirect.txt");

        repl.handle_redirection(&format!("echo redirected > {}", target.display()))
            .await
            .expect("redirect");

        let content = std::fs::read_to_string(&target).expect("read file");
        assert_eq!(content, "redirected\n");
    }
}
