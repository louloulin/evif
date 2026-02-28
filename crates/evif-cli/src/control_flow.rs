// Shell 控制流支持 - if/for/while 语句

use anyhow::Result;
use std::collections::HashMap;

/// 控制流执行器
pub struct ControlFlowExecutor {
    variables: HashMap<String, String>,
}

impl ControlFlowExecutor {
    /// 创建新的控制流执行器
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// 设置变量
    pub fn set_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// 获取变量
    pub fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.get(name).cloned().or_else(|| std::env::var(name).ok())
    }

    /// 列出所有变量
    pub fn list_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// 展开变量
    pub fn expand_variables(&self, input: &str) -> String {
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
                    let value = self.get_variable(&var_name).unwrap_or_default();
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
                        _ => self.get_variable(&var_name).unwrap_or_default(),
                    };
                    result.push_str(&value);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// 解析并执行脚本
    pub async fn execute_script<F, Fut>(
        &mut self,
        script: &str,
        mut execute_command: F,
    ) -> Result<()>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let statements = self.parse_statements(script)?;

        for statement in statements {
            self.execute_statement(&statement, &mut execute_command).await?;
        }

        Ok(())
    }

    /// 解析语句列表
    fn parse_statements(&self, script: &str) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        let mut lines: Vec<&str> = script.lines().collect();
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

        // if 语句: if condition { commands } [else { commands }]
        if let Some(rest) = line.strip_prefix("if ") {
            let (condition, body) = self.parse_block(lines, i, "if", "endif")?;
            return Ok(Some(Statement::If(condition, body)));
        }

        // for 循环: for var in list { commands }
        if let Some(rest) = line.strip_prefix("for ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "in" {
                let var_name = parts[0].to_string();
                let list_expr = parts[2..].join(" ");
                let (_list_check, body) = self.parse_block(lines, i, "for", "endfor")?;
                return Ok(Some(Statement::For(var_name, list_expr, body)));
            }
        }

        // while 循环: while condition { commands }
        if let Some(rest) = line.strip_prefix("while ") {
            let (condition, body) = self.parse_block(lines, i, "while", "endwhile")?;
            return Ok(Some(Statement::While(condition, body)));
        }

        Ok(None)
    }

    /// 解析代码块
    fn parse_block(&self, lines: &[&str], i: &mut usize, start_kw: &str, end_kw: &str) -> Result<(String, Vec<Statement>)> {
        let first_line = lines[*i - 1].trim();
        let mut block_lines = Vec::new();

        // 检查是否有花括号
        if first_line.contains('{') {
            // 单行 if/for/while { ... }
            let open_brace = first_line.find('{').unwrap();
            let condition = first_line[start_kw.len() + 1..open_brace].trim().to_string();

            // 收集块内容
            let mut brace_count = 1;
            let mut block_content = String::new();

            if let Some(rest) = first_line.get(open_brace + 1..) {
                if rest.contains('}') {
                    let close_brace = rest.find('}').unwrap();
                    block_content.push_str(rest[..close_brace].trim());
                    brace_count = 0;
                } else {
                    block_content.push_str(rest.trim());
                }
            }

            while brace_count > 0 && *i < lines.len() {
                let line = lines[*i].trim();
                *i += 1;

                if line.contains('}') {
                    let close_pos = line.find('}').unwrap();
                    block_content.push_str(" ");
                    block_content.push_str(&line[..close_pos]);
                    brace_count = 0;
                    break;
                } else {
                    block_content.push_str("\n");
                    block_content.push_str(line);
                }
            }

            let body_statements = self.parse_statements(&block_content)?;
            return Ok((condition, body_statements));
        }

        // 多行块语法 (使用 endif/endfor/endwhile)
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
                    self.set_variable(var_name.clone(), value);
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

    /// 评估条件表达式
    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        let condition = self.expand_variables(condition.trim());
        let condition = condition.trim();

        // 文件测试: -f file, -d dir, -e path
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

        // 简单的真值检查（非空字符串为真）
        Ok(!condition.is_empty())
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
    async fn test_simple_if() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
if "test" == "test" {
    echo hello
}
"#;

        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| {
            commands.push(cmd);
            async { Ok(()) }
        }).await.unwrap();

        assert_eq!(commands, vec!["echo hello"]);
    }

    #[tokio::test]
    async fn test_for_loop() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
for i in 1 2 3 {
    echo $i
}
"#;

        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| {
            commands.push(cmd);
            async { Ok(()) }
        }).await.unwrap();

        assert_eq!(commands, vec!["echo 1", "echo 2", "echo 3"]);
    }

    #[tokio::test]
    async fn test_variable_expansion() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("NAME".to_string(), "world".to_string());

        let expanded = executor.expand_variables("echo $NAME");
        assert_eq!(expanded, "echo world");

        let expanded2 = executor.expand_variables("echo ${NAME}");
        assert_eq!(expanded2, "echo world");
    }
}
