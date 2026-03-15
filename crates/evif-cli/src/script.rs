// 脚本执行支持 - .as (AGFS Script) 文件
// 支持 if/for/while 控制流和变量替换

use crate::commands::EvifCommand;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// 脚本执行器
pub struct ScriptExecutor {
    variables: HashMap<String, String>,
    server: String,
}

impl ScriptExecutor {
    /// 创建新的脚本执行器
    pub fn new(server: String) -> Self {
        Self {
            variables: HashMap::new(),
            server,
        }
    }

    /// 执行脚本文件
    pub async fn execute_file(&mut self, path: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read script file: {}", e))?;

        self.execute_script(&content).await
    }

    /// 执行脚本内容（支持控制流）
    pub async fn execute_script(&mut self, script: &str) -> Result<()> {
        println!("Executing AGFS Script with Control Flow Support...");

        // 收集要执行的命令，然后逐个执行
        let mut commands_to_execute = Vec::new();
        {
            let mut parser = ScriptParser::new(&mut self.variables);
            commands_to_execute = parser.parse_commands(script)?;
        }

        // 逐个执行命令
        for cmd in commands_to_execute {
            self.execute_single_command(&cmd).await?;
        }

        Ok(())
    }

    /// 执行单个命令
    async fn execute_single_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "echo" => {
                println!("{}", args.join(" "));
            }
            "sleep" => {
                if let Some(seconds) = args.first() {
                    let secs: u64 = seconds
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid sleep duration: {}", seconds))?;
                    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
                }
            }
            "set" => {
                if args.len() >= 2 {
                    let key = args[0].to_string();
                    let value = args[1..].join(" ");
                    self.variables.insert(key, value);
                }
            }
            _ => {
                // EVIF 命令需要通过 REST API 执行
                // 这里提供占位符实现，实际集成需要 EvifClient
                println!("Executing EVIF command: {}", command);
                println!("Note: Full EVIF command integration requires EvifClient");
            }
        }

        Ok(())
    }

    /// 设置变量
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// 获取变量
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// 列出所有变量
    pub fn list_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// 展开变量
    pub fn expand_variables(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    let mut var_name = String::new();
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }
                    let value = self
                        .get_variable(&var_name)
                        .cloned()
                        .or_else(|| std::env::var(&var_name).ok())
                        .unwrap_or_default();
                    result.push_str(&value);
                } else {
                    let mut var_name = String::new();
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c.is_alphanumeric() || inner_c == '_' || inner_c == '?' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let value = match var_name.as_str() {
                        "?" => "0".to_string(),
                        "$" => std::process::id().to_string(),
                        "0" => "evif".to_string(),
                        _ => self
                            .get_variable(&var_name)
                            .cloned()
                            .or_else(|| std::env::var(&var_name).ok())
                            .unwrap_or_default(),
                    };
                    result.push_str(&value);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// 使用 EvifCommand 执行脚本文件（静态方法，用于 REPL 集成）
    pub async fn execute_script_with_client(script_path: &str, client: &EvifCommand) -> Result<()> {
        let content = tokio::fs::read_to_string(script_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read script file: {}", e))?;

        let mut variables = HashMap::new();
        let mut parser = ScriptParser::new(&mut variables);
        let commands = parser.parse_commands(&content)?;

        // 使用 EvifCommand 执行每个命令
        for cmd in commands {
            // 解析命令
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let cmd_name = parts[0];

            // 根据命令类型调用 EvifCommand 的方法
            match cmd_name {
                "echo" => {
                    println!("{}", parts[1..].join(" "));
                }
                "ls" => {
                    let path = parts
                        .get(1)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "/".to_string());
                    if let Err(e) = client.ls(Some(path), false, false).await {
                        eprintln!("Error: {}", e);
                    }
                }
                "cat" => {
                    if let Some(path) = parts.get(1) {
                        if let Err(e) = client.cat(path.to_string()).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "write" => {
                    if parts.len() >= 3 {
                        let path = parts[1].to_string();
                        let content = parts[2..].join(" ");
                        if let Err(e) = client.write(path, content, false).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "mkdir" => {
                    if let Some(path) = parts.get(1) {
                        if let Err(e) = client.mkdir(path.to_string(), false).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "rm" => {
                    if let Some(path) = parts.get(1) {
                        if let Err(e) = client.rm(path.to_string(), false).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "mv" => {
                    if let (Some(src), Some(dst)) = (parts.get(1), parts.get(2)) {
                        if let Err(e) = client.mv(src.to_string(), dst.to_string()).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "cp" => {
                    if let (Some(src), Some(dst)) = (parts.get(1), parts.get(2)) {
                        if let Err(e) = client.cp(src.to_string(), dst.to_string()).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "touch" => {
                    if let Some(path) = parts.get(1) {
                        if let Err(e) = client.touch(path.to_string()).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                "sleep" => {
                    if let Some(seconds) = parts.get(1) {
                        if let Ok(secs) = seconds.parse::<u64>() {
                            tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
                        }
                    }
                }
                _ => {
                    eprintln!("Unknown script command: {}", cmd_name);
                }
            }
        }

        Ok(())
    }
}

/// 脚本解析器
struct ScriptParser<'a> {
    variables: &'a mut HashMap<String, String>,
}

impl<'a> ScriptParser<'a> {
    fn new(variables: &'a mut HashMap<String, String>) -> Self {
        Self { variables }
    }

    /// 展开变量
    fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    let mut var_name = String::new();
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }
                    let value = self
                        .variables
                        .get(&var_name)
                        .cloned()
                        .or_else(|| std::env::var(&var_name).ok())
                        .unwrap_or_default();
                    result.push_str(&value);
                } else {
                    let mut var_name = String::new();
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c.is_alphanumeric() || inner_c == '_' || inner_c == '?' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let value = match var_name.as_str() {
                        "?" => "0".to_string(),
                        "$" => std::process::id().to_string(),
                        "0" => "evif".to_string(),
                        _ => self
                            .variables
                            .get(&var_name)
                            .cloned()
                            .or_else(|| std::env::var(&var_name).ok())
                            .unwrap_or_default(),
                    };
                    result.push_str(&value);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// 评估条件
    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        let condition = self.expand_variables(condition.trim());
        let condition = condition.trim();

        // 文件测试
        if let Some(rest) = condition.strip_prefix("-f ") {
            let path = rest.trim();
            return Ok(std::path::Path::new(path).is_file());
        }
        if let Some(rest) = condition.strip_prefix("-d ") {
            let path = rest.trim();
            return Ok(std::path::Path::new(path).is_dir());
        }
        if let Some(rest) = condition.strip_prefix("-e ") {
            let path = rest.trim();
            return Ok(std::path::Path::new(path).exists());
        }

        // 字符串比较
        if condition.contains("==") {
            let parts: Vec<&str> = condition.splitn(2, "==").collect();
            if parts.len() == 2 {
                return Ok(parts[0].trim() == parts[1].trim());
            }
        }
        if condition.contains("!=") {
            let parts: Vec<&str> = condition.splitn(2, "!=").collect();
            if parts.len() == 2 {
                return Ok(parts[0].trim() != parts[1].trim());
            }
        }

        // 数值比较
        if condition.contains("<=") {
            let parts: Vec<&str> = condition.splitn(2, "<=").collect();
            if parts.len() == 2 {
                let left = parts[0].trim().parse::<i64>().unwrap_or(i64::MIN);
                let right = parts[1].trim().parse::<i64>().unwrap_or(i64::MAX);
                return Ok(left <= right);
            }
        }
        if condition.contains(">=") {
            let parts: Vec<&str> = condition.splitn(2, ">=").collect();
            if parts.len() == 2 {
                let left = parts[0].trim().parse::<i64>().unwrap_or(i64::MIN);
                let right = parts[1].trim().parse::<i64>().unwrap_or(i64::MAX);
                return Ok(left >= right);
            }
        }
        if condition.contains('<') {
            let parts: Vec<&str> = condition.splitn(2, '<').collect();
            if parts.len() == 2 {
                let left = parts[0].trim().parse::<i64>().unwrap_or(i64::MIN);
                let right = parts[1].trim().parse::<i64>().unwrap_or(i64::MAX);
                return Ok(left < right);
            }
        }
        if condition.contains('>') {
            let parts: Vec<&str> = condition.splitn(2, '>').collect();
            if parts.len() == 2 {
                let left = parts[0].trim().parse::<i64>().unwrap_or(i64::MIN);
                let right = parts[1].trim().parse::<i64>().unwrap_or(i64::MAX);
                return Ok(left > right);
            }
        }

        // 简单的真值检查
        Ok(!condition.is_empty())
    }

    /// 执行脚本
    async fn execute<F, Fut>(mut self, script: &str, mut execute_command: F) -> Result<()>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let statements = self.parse_statements(script)?;

        for statement in statements {
            self.execute_statement(&statement, &mut execute_command)
                .await?;
        }

        Ok(())
    }

    /// 解析并展开所有命令（用于在执行前展开变量）
    fn parse_commands(&mut self, script: &str) -> Result<Vec<String>> {
        let statements = self.parse_statements(script)?;
        let mut commands = Vec::new();

        for statement in statements {
            self.collect_commands(&statement, &mut commands)?;
        }

        Ok(commands)
    }

    /// 收集语句中的所有命令
    fn collect_commands(
        &mut self,
        statement: &Statement,
        commands: &mut Vec<String>,
    ) -> Result<()> {
        match statement {
            Statement::Command(cmd) => {
                let expanded = self.expand_variables(cmd);
                commands.push(expanded);
            }
            Statement::If(condition, then_body) => {
                if self.evaluate_condition(condition)? {
                    for stmt in then_body {
                        self.collect_commands(stmt, commands)?;
                    }
                }
            }
            Statement::For(var_name, list_expr, body) => {
                let expanded_list = self.expand_variables(list_expr);
                let values: Vec<String> = expanded_list
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                for value in values {
                    self.variables.insert(var_name.clone(), value);
                    for stmt in body {
                        self.collect_commands(stmt, commands)?;
                    }
                }
            }
            Statement::While(condition, body) => {
                // While 循环需要运行时评估，这里只执行一次
                if self.evaluate_condition(condition)? {
                    for stmt in body {
                        self.collect_commands(stmt, commands)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 解析语句列表
    fn parse_statements(&self, script: &str) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        let lines: Vec<&str> = script.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            i += 1;

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 检查是否是控制流语句
            if let Some(statement) = self.parse_control_flow(&lines, &mut i)? {
                statements.push(statement);
            } else {
                // 普通命令
                statements.push(Statement::Command(line.to_string()));
            }
        }

        Ok(statements)
    }

    /// 解析控制流语句
    fn parse_control_flow(&self, lines: &[&str], i: &mut usize) -> Result<Option<Statement>> {
        let line = lines[*i - 1].trim();

        // if 语句
        if let Some(_rest) = line.strip_prefix("if ") {
            let (condition, body) = self.parse_block(lines, i, "if", "endif")?;
            return Ok(Some(Statement::If(condition, body)));
        }

        // for 循环
        if let Some(rest) = line.strip_prefix("for ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "in" {
                let var_name = parts[0].to_string();
                let list_expr = parts[2..].join(" ");
                let body = self.parse_block(lines, i, "for", "endfor")?.1;
                return Ok(Some(Statement::For(var_name, list_expr, body)));
            }
        }

        // while 循环
        if let Some(_rest) = line.strip_prefix("while ") {
            let (condition, body) = self.parse_block(lines, i, "while", "endwhile")?;
            return Ok(Some(Statement::While(condition, body)));
        }

        Ok(None)
    }

    /// 解析代码块
    fn parse_block(
        &self,
        lines: &[&str],
        i: &mut usize,
        start_kw: &str,
        end_kw: &str,
    ) -> Result<(String, Vec<Statement>)> {
        let first_line = lines[*i - 1].trim();
        let mut block_lines = Vec::new();

        // 检查是否有花括号
        if first_line.contains('{') {
            let open_brace = first_line.find('{').unwrap();
            let condition = first_line[start_kw.len() + 1..open_brace]
                .trim()
                .to_string();

            let mut block_content = String::new();

            if let Some(rest) = first_line.get(open_brace + 1..) {
                if rest.contains('}') {
                    let close_brace = rest.find('}').unwrap();
                    block_content.push_str(rest[..close_brace].trim());
                } else {
                    block_content.push_str(rest.trim());
                }
            }

            while *i < lines.len() {
                let line = lines[*i].trim();
                *i += 1;

                if line.contains('}') {
                    let close_pos = line.find('}').unwrap();
                    block_content.push_str(" ");
                    block_content.push_str(&line[..close_pos]);
                    break;
                } else {
                    block_content.push_str("\n");
                    block_content.push_str(line);
                }
            }

            let body_statements = self.parse_statements(&block_content)?;
            return Ok((condition, body_statements));
        }

        // 多行块语法
        let condition = first_line[start_kw.len() + 1..].trim().to_string();

        while *i < lines.len() {
            let line = lines[*i].trim();
            *i += 1;

            if line == end_kw {
                break;
            }

            block_lines.push(line);
        }

        let block_content = block_lines.join("\n");
        let body_statements = self.parse_statements(&block_content)?;

        Ok((condition, body_statements))
    }

    /// 执行语句
    async fn execute_statement<F, Fut>(
        &mut self,
        statement: &Statement,
        execute_command: &mut F,
    ) -> Result<()>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        match statement {
            Statement::Command(cmd) => {
                let expanded = self.expand_variables(cmd);
                execute_command(expanded).await?;
            }
            Statement::If(condition, then_body) => {
                if self.evaluate_condition(condition)? {
                    for stmt in then_body {
                        self.execute_statement(stmt, execute_command).await?;
                    }
                }
            }
            Statement::For(var_name, list_expr, body) => {
                let expanded_list = self.expand_variables(list_expr);
                let values: Vec<String> = expanded_list
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                for value in values {
                    self.variables.insert(var_name.clone(), value);
                    for stmt in body {
                        self.execute_statement(stmt, execute_command).await?;
                    }
                }
            }
            Statement::While(condition, body) => {
                while self.evaluate_condition(condition)? {
                    for stmt in body {
                        self.execute_statement(stmt, execute_command).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// 语句类型
#[derive(Debug, Clone)]
enum Statement {
    Command(String),
    If(String, Vec<Statement>),
    For(String, String, Vec<Statement>),
    While(String, Vec<Statement>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_variable_assignment() {
        let mut executor = ScriptExecutor::new("localhost:8080".to_string());

        executor.set_variable("PATH".to_string(), "/tmp".to_string());
        assert_eq!(executor.get_variable("PATH"), Some(&"/tmp".to_string()));
    }

    #[tokio::test]
    async fn test_variable_expansion() {
        let mut executor = ScriptExecutor::new("localhost:8080".to_string());
        executor.set_variable("NAME".to_string(), "value".to_string());

        let expanded = executor.expand_variables("ls $NAME");
        assert_eq!(expanded, "ls value");
    }

    #[tokio::test]
    async fn test_execute_simple_script() {
        let mut executor = ScriptExecutor::new("localhost:8080".to_string());

        let script = r#"
# This is a comment
echo "Testing"
"#;

        let result = executor.execute_script(script).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_if_statement() {
        let mut executor = ScriptExecutor::new("localhost:8080".to_string());

        let script = r#"
if "test" == "test" {
    echo hello
}
"#;

        let result = executor.execute_script(script).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_for_loop() {
        let mut executor = ScriptExecutor::new("localhost:8080".to_string());

        let script = r#"
for i in 1 2 3 {
    echo $i
}
"#;

        let result = executor.execute_script(script).await;
        assert!(result.is_ok());
    }
}
