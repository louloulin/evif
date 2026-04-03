// Shell 控制流支持 - if/for/while/fn/break/continue/return/算术/字符串操作

use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// 控制流信号 - break/continue/return
#[derive(Debug, Clone, PartialEq)]
pub enum FlowSignal {
    None,
    Break(usize),
    Continue(usize),
    Return(Option<String>),
}

/// 函数定义
#[derive(Debug, Clone)]
struct Function {
    params: Vec<String>,
    body: Vec<Statement>,
}

/// 控制流执行器
pub struct ControlFlowExecutor {
    variables: HashMap<String, String>,
    functions: HashMap<String, Function>,
}

impl ControlFlowExecutor {
    /// 创建新的控制流执行器
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
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

    /// 删除变量
    pub fn remove_variable(&mut self, name: &str) {
        self.variables.remove(name);
    }

    /// 列出所有变量
    #[allow(dead_code)]
    pub fn list_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// 展开变量 - 支持 $VAR, ${VAR}, ${#VAR}, ${VAR:-default}, ${VAR:+alt}, ${VAR:offset:len}, $((expr))
    pub fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let chars_vec: Vec<char> = input.chars().collect();
        let mut pos = 0;

        while pos < chars_vec.len() {
            let c = chars_vec[pos];
            pos += 1;

            if c == '$' {
                // Check next two chars for $(( pattern
                let next1 = chars_vec.get(pos).copied();
                let next2 = chars_vec.get(pos + 1).copied();

                match next1 {
                    // $((arithmetic))
                    Some('(') if next2 == Some('(') => {
                        pos += 2; // consume ((
                        // Collect everything until matching ))
                        let mut expr = String::new();
                        // We need to find )) that closes the $(( — track nesting of inner ()
                        let mut paren_depth = 0;
                        while pos < chars_vec.len() {
                            let ec = chars_vec[pos];
                            if ec == '(' {
                                paren_depth += 1;
                                expr.push(ec);
                                pos += 1;
                            } else if ec == ')' {
                                if paren_depth > 0 {
                                    paren_depth -= 1;
                                    expr.push(ec);
                                    pos += 1;
                                } else if chars_vec.get(pos + 1) == Some(&')') {
                                    // Found closing ))
                                    pos += 2;
                                    break;
                                } else {
                                    expr.push(ec);
                                    pos += 1;
                                }
                            } else {
                                expr.push(ec);
                                pos += 1;
                            }
                        }
                        let val = self.evaluate_arithmetic(&expr);
                        result.push_str(&val);
                    }
                    // ${...} - brace expansion with operations
                    Some('{') => {
                        pos += 1; // consume {
                        let mut var_expr = String::new();
                        let mut depth = 1;
                        while pos < chars_vec.len() {
                            let ec = chars_vec[pos];
                            pos += 1;
                            if ec == '{' {
                                depth += 1;
                                var_expr.push(ec);
                            } else if ec == '}' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                var_expr.push(ec);
                            } else {
                                var_expr.push(ec);
                            }
                        }
                        result.push_str(&self.expand_brace_var(&var_expr));
                    }
                    // $VAR - simple variable
                    Some(_) => {
                        let mut var_name = String::new();
                        while pos < chars_vec.len() {
                            let nc = chars_vec[pos];
                            if nc.is_alphanumeric() || nc == '_' || nc == '?' {
                                var_name.push(nc);
                                pos += 1;
                            } else {
                                break;
                            }
                        }
                        let value = self.resolve_special_var(&var_name);
                        result.push_str(&value);
                    }
                    None => result.push('$'),
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Resolve special and regular variables
    fn resolve_special_var(&self, name: &str) -> String {
        match name {
            "?" => "0".to_string(),
            "$" => std::process::id().to_string(),
            "0" => "evif".to_string(),
            "" => String::new(),
            _ => self.get_variable(name).unwrap_or_default(),
        }
    }

    /// Expand ${...} variable expression with operations
    fn expand_brace_var(&self, expr: &str) -> String {
        // ${#VAR} - string length
        if let Some(var_name) = expr.strip_prefix('#') {
            let val = self.resolve_special_var(var_name);
            return val.len().to_string();
        }

        // ${VAR:-default} - use default if unset/empty
        if let Some(idx) = expr.find(":-") {
            let var_name = &expr[..idx];
            let default = &expr[idx + 2..];
            let val = self.resolve_special_var(var_name);
            if val.is_empty() {
                return default.to_string();
            }
            return val;
        }

        // ${VAR:+alternative} - use alternative if set
        if let Some(idx) = expr.find(":+") {
            let var_name = &expr[..idx];
            let alternative = &expr[idx + 2..];
            let val = self.resolve_special_var(var_name);
            if val.is_empty() {
                return String::new();
            }
            return alternative.to_string();
        }

        // ${VAR:?error} - error if unset
        if let Some(idx) = expr.find(":?") {
            let var_name = &expr[..idx];
            let error_msg = &expr[idx + 2..];
            let val = self.resolve_special_var(var_name);
            if val.is_empty() {
                eprintln!("Error: {}: {}", var_name, error_msg);
                return String::new();
            }
            return val;
        }

        // ${VAR:offset:length} - substring
        if let Some(first_colon) = expr.find(':') {
            let rest = &expr[first_colon + 1..];
            // Check if it's offset:length format (not :- which was handled above)
            if rest.contains(':') {
                let var_name = &expr[..first_colon];
                if let Some(second_colon) = rest.find(':') {
                    let offset_str = &rest[..second_colon];
                    let length_str = &rest[second_colon + 1..];
                    if let (Ok(offset), Ok(length)) = (offset_str.parse::<usize>(), length_str.parse::<usize>()) {
                        let val = self.resolve_special_var(var_name);
                        let chars: Vec<char> = val.chars().collect();
                        if offset < chars.len() {
                            let end = std::cmp::min(offset + length, chars.len());
                            return chars[offset..end].iter().collect();
                        }
                        return String::new();
                    }
                }
            }
            // ${VAR:offset} - substring from offset
            if let Ok(offset) = rest.parse::<usize>() {
                let var_name = &expr[..first_colon];
                let val = self.resolve_special_var(var_name);
                let chars: Vec<char> = val.chars().collect();
                if offset < chars.len() {
                    return chars[offset..].iter().collect();
                }
                return String::new();
            }
        }

        // ${VAR} - simple expansion
        self.resolve_special_var(expr)
    }

    /// Evaluate arithmetic expression: +, -, *, /, %, **, (), variables
    fn evaluate_arithmetic(&self, expr: &str) -> String {
        let expanded = self.expand_variables(expr.trim());
        match self.parse_arith_expr(&expanded) {
            Ok(val) => {
                if val == (val as i64) as f64 {
                    (val as i64).to_string()
                } else {
                    format!("{}", val)
                }
            }
            Err(_) => "0".to_string(),
        }
    }

    /// Recursive descent arithmetic parser
    fn parse_arith_expr(&self, expr: &str) -> Result<f64, ()> {
        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(0.0);
        }

        let tokens = self.tokenize_arith(expr)?;
        let mut pos = 0;
        let result = self.parse_arith_additive(&tokens, &mut pos)?;
        Ok(result)
    }

    fn tokenize_arith(&self, expr: &str) -> Result<Vec<ArithToken>, ()> {
        let mut tokens = Vec::new();
        let mut chars = expr.chars().peekable();

        while let Some(&c) = chars.peek() {
            match c {
                ' ' | '\t' => { chars.next(); }
                '+' => { chars.next(); tokens.push(ArithToken::Plus); }
                '-' => { chars.next(); tokens.push(ArithToken::Minus); }
                '*' => {
                    chars.next();
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        tokens.push(ArithToken::Power);
                    } else {
                        tokens.push(ArithToken::Mul);
                    }
                }
                '/' => { chars.next(); tokens.push(ArithToken::Div); }
                '%' => { chars.next(); tokens.push(ArithToken::Mod); }
                '(' => { chars.next(); tokens.push(ArithToken::LParen); }
                ')' => { chars.next(); tokens.push(ArithToken::RParen); }
                '0'..='9' | '.' => {
                    let mut num = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_ascii_digit() || nc == '.' {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(ArithToken::Num(num.parse::<f64>().map_err(|_| ())?));
                }
                _ if c.is_alphanumeric() || c == '_' => {
                    let mut name = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_alphanumeric() || nc == '_' {
                            name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let val: f64 = self.resolve_special_var(&name).parse().unwrap_or(0.0);
                    tokens.push(ArithToken::Num(val));
                }
                _ => return Err(()),
            }
        }
        Ok(tokens)
    }

    fn parse_arith_additive(&self, tokens: &[ArithToken], pos: &mut usize) -> Result<f64, ()> {
        let mut left = self.parse_arith_multiplicative(tokens, pos)?;
        while *pos < tokens.len() {
            match &tokens[*pos] {
                ArithToken::Plus => { *pos += 1; left += self.parse_arith_multiplicative(tokens, pos)?; }
                ArithToken::Minus => { *pos += 1; left -= self.parse_arith_multiplicative(tokens, pos)?; }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_arith_multiplicative(&self, tokens: &[ArithToken], pos: &mut usize) -> Result<f64, ()> {
        let mut left = self.parse_arith_power(tokens, pos)?;
        while *pos < tokens.len() {
            match &tokens[*pos] {
                ArithToken::Mul => { *pos += 1; left *= self.parse_arith_power(tokens, pos)?; }
                ArithToken::Div => { *pos += 1; let right = self.parse_arith_power(tokens, pos)?; if right != 0.0 { left = ((left as i64) / (right as i64)) as f64; } }
                ArithToken::Mod => { *pos += 1; let right = self.parse_arith_power(tokens, pos)?; if right != 0.0 { left = (left as i64 % right as i64) as f64; } }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_arith_power(&self, tokens: &[ArithToken], pos: &mut usize) -> Result<f64, ()> {
        let base = self.parse_arith_unary(tokens, pos)?;
        if *pos < tokens.len() && matches!(&tokens[*pos], ArithToken::Power) {
            *pos += 1;
            let exp = self.parse_arith_power(tokens, pos)?; // right-associative
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_arith_unary(&self, tokens: &[ArithToken], pos: &mut usize) -> Result<f64, ()> {
        if *pos < tokens.len() {
            match &tokens[*pos] {
                ArithToken::Minus => { *pos += 1; Ok(-self.parse_arith_primary(tokens, pos)?) }
                ArithToken::Plus => { *pos += 1; self.parse_arith_primary(tokens, pos) }
                _ => self.parse_arith_primary(tokens, pos),
            }
        } else {
            Ok(0.0)
        }
    }

    fn parse_arith_primary(&self, tokens: &[ArithToken], pos: &mut usize) -> Result<f64, ()> {
        if *pos >= tokens.len() {
            return Ok(0.0);
        }
        match &tokens[*pos] {
            ArithToken::Num(n) => { let v = *n; *pos += 1; Ok(v) }
            ArithToken::LParen => {
                *pos += 1;
                let v = self.parse_arith_additive(tokens, pos)?;
                if *pos < tokens.len() && matches!(&tokens[*pos], ArithToken::RParen) {
                    *pos += 1;
                }
                Ok(v)
            }
            _ => Err(()),
        }
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
            let signal = self.execute_statement(&statement, &mut execute_command).await?;
            if signal != FlowSignal::None {
                break;
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

        // fn 语句: fn name(params) { body }
        if let Some(_rest) = line.strip_prefix("fn ") {
            let (name, params, body) = self.parse_function_def(lines, i)?;
            return Ok(Some(Statement::FnDef(name, params, body)));
        }

        // if 语句: if condition { commands } [else { commands }]
        if let Some(_rest) = line.strip_prefix("if ") {
            let (condition, body) = self.parse_block(lines, i, "if", "endif")?;
            // Check for else
            let else_body = if *i < lines.len() {
                let next_line = lines[*i].trim();
                if next_line.starts_with("} else") || next_line == "else" {
                    *i += 1;
                    if next_line.contains('{') {
                        // inline else { ... }
                        let mut else_content = String::new();
                        while *i < lines.len() {
                            let l = lines[*i].trim();
                            *i += 1;
                            if l.contains('}') {
                                let pos = l.find('}').unwrap();
                                else_content.push_str(&l[..pos]);
                                break;
                            } else {
                                else_content.push_str(l);
                                else_content.push('\n');
                            }
                        }
                        Some(self.parse_statements(&else_content)?)
                    } else {
                        Some(self.parse_until_end(lines, i, "endif")?)
                    }
                } else {
                    None
                }
            } else {
                None
            };
            return Ok(Some(Statement::If(condition, body, else_body)));
        }

        // for 循环: for var in list { commands }
        if let Some(rest) = line.strip_prefix("for ") {
            // Strip { and everything after it to avoid including it in the list
            let rest_before_brace = if let Some(bp) = rest.find('{') {
                &rest[..bp]
            } else {
                rest
            };
            let parts: Vec<&str> = rest_before_brace.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "in" {
                let var_name = parts[0].to_string();
                let list_expr = parts[2..].join(" ");
                let (_list_check, body) = self.parse_block(lines, i, "for", "endfor")?;
                return Ok(Some(Statement::For(var_name, list_expr, body)));
            }
        }

        // while 循环: while condition { commands }
        if let Some(_rest) = line.strip_prefix("while ") {
            let (condition, body) = self.parse_block(lines, i, "while", "endwhile")?;
            return Ok(Some(Statement::While(condition, body)));
        }

        Ok(None)
    }

    /// Parse function definition
    fn parse_function_def(&self, lines: &[&str], i: &mut usize) -> Result<(String, Vec<String>, Vec<Statement>)> {
        let first_line = lines[*i - 1].trim();
        let rest = first_line.strip_prefix("fn ").unwrap().trim();

        // Parse: fn name(params) { body } or fn name(params)
        let open_paren = rest.find('(').unwrap_or(rest.len());
        let name = rest[..open_paren].trim().to_string();

        let params = if open_paren < rest.len() {
            let close_paren = rest[open_paren..].find(')').unwrap_or(0);
            let params_str = &rest[open_paren + 1..open_paren + close_paren];
            params_str.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
        } else {
            Vec::new()
        };

        let (_, body) = self.parse_block(lines, i, "fn", "endfn")?;
        Ok((name, params, body))
    }

    /// Parse until end keyword
    fn parse_until_end(&self, lines: &[&str], i: &mut usize, end_kw: &str) -> Result<Vec<Statement>> {
        let mut block_lines = Vec::new();
        while *i < lines.len() {
            let line = lines[*i].trim();
            *i += 1;
            if line == end_kw {
                break;
            }
            block_lines.push(line);
        }
        self.parse_statements(&block_lines.join("\n"))
    }

    /// 解析代码块
    fn parse_block(&self, lines: &[&str], i: &mut usize, start_kw: &str, end_kw: &str) -> Result<(String, Vec<Statement>)> {
        let first_line = lines[*i - 1].trim();

        // 检查是否有花括号
        if first_line.contains('{') {
            let open_brace = first_line.find('{').unwrap();
            // For "fn", the condition/params are before the {
            let kw_end = if start_kw == "fn" {
                // fn name(params) { — everything before { is the signature, not condition
                String::new()
            } else {
                first_line[start_kw.len() + 1..open_brace].trim().to_string()
            };

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

                // Track braces left-to-right to handle "} else {" correctly
                let mut found_outer_close = false;
                let mut close_pos = 0;
                let chars: Vec<char> = line.chars().collect();
                let mut cp = 0;
                while cp < chars.len() {
                    match chars[cp] {
                        '{' => {
                            brace_count += 1;
                            cp += 1;
                        }
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                found_outer_close = true;
                                close_pos = cp;
                                break;
                            }
                            cp += 1;
                        }
                        _ => cp += 1,
                    }
                }

                if found_outer_close {
                    block_content.push(' ');
                    block_content.push_str(&line[..close_pos]);
                    let after_close = line[close_pos + 1..].trim();
                    if start_kw == "if" && after_close.starts_with("else") {
                        *i -= 1;
                    }
                    break;
                } else {
                    block_content.push('\n');
                    block_content.push_str(line);
                }
            }

            let body_statements = self.parse_statements(&block_content)?;
            return Ok((kw_end, body_statements));
        }

        // 多行块语法 (使用 endfn/endif/endfor/endwhile)
        let condition = first_line[start_kw.len() + 1..].trim().to_string();
        let body = self.parse_until_end(lines, i, end_kw)?;
        Ok((condition, body))
    }

    /// 执行语句
    fn execute_statement<'a, F, Fut>(
        &'a mut self,
        statement: &'a Statement,
        execute_command: &'a mut F,
    ) -> Pin<Box<dyn Future<Output = Result<FlowSignal>> + 'a>>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Box::pin(async move {
        match statement {
            Statement::Command(cmd) => {
                let expanded = self.expand_variables(cmd);
                // Handle break/continue/return as commands
                if expanded == "break" {
                    return Ok(FlowSignal::Break(1));
                }
                if let Some(stripped) = expanded.strip_prefix("break ") {
                    let level: usize = stripped.trim().parse().unwrap_or(1);
                    return Ok(FlowSignal::Break(level));
                }
                if expanded == "continue" {
                    return Ok(FlowSignal::Continue(1));
                }
                if let Some(stripped) = expanded.strip_prefix("continue ") {
                    let level: usize = stripped.trim().parse().unwrap_or(1);
                    return Ok(FlowSignal::Continue(level));
                }
                if expanded == "return" {
                    return Ok(FlowSignal::Return(None));
                }
                if let Some(stripped) = expanded.strip_prefix("return ") {
                    let val = stripped.trim().to_string();
                    return Ok(FlowSignal::Return(Some(val)));
                }
                // Handle `set VAR VALUE` to update executor's own variables
                if let Some(stripped) = expanded.strip_prefix("set ") {
                    let rest = stripped.trim();
                    let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
                    if parts.len() >= 2 {
                        self.set_variable(parts[0].to_string(), parts[1].to_string());
                    } else if parts.len() == 1 {
                        self.set_variable(parts[0].to_string(), String::new());
                    }
                    return Ok(FlowSignal::None);
                }
                // Handle `export VAR=VALUE` style
                if expanded.contains('=') && !expanded.starts_with("echo") {
                    let eq_pos = expanded.find('=').unwrap();
                    if eq_pos > 0 {
                        let var_name = &expanded[..eq_pos];
                        // Only treat as assignment if var name is a valid identifier
                        if var_name.chars().all(|c| c.is_alphanumeric() || c == '_') && !var_name.chars().next().unwrap().is_ascii_digit() {
                            let value = expanded[eq_pos + 1..].to_string();
                            self.set_variable(var_name.to_string(), value);
                            return Ok(FlowSignal::None);
                        }
                    }
                }
                // Check for function call
                let parts: Vec<&str> = expanded.split_whitespace().collect();
                if let Some(fname) = parts.first() {
                    if self.functions.contains_key(*fname) {
                        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        return self.execute_function_call(fname, &args, execute_command).await;
                    }
                }
                execute_command(expanded).await?;
                Ok(FlowSignal::None)
            }
            Statement::If(condition, then_body, else_body) => {
                if self.evaluate_condition(condition)? {
                    for stmt in then_body {
                        let signal = self.execute_statement(stmt, execute_command).await?;
                        if signal != FlowSignal::None {
                            return Ok(signal);
                        }
                    }
                } else if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        let signal = self.execute_statement(stmt, execute_command).await?;
                        if signal != FlowSignal::None {
                            return Ok(signal);
                        }
                    }
                }
                Ok(FlowSignal::None)
            }
            Statement::For(var_name, list_expr, body) => {
                let expanded_list = self.expand_variables(list_expr);
                let values: Vec<String> = expanded_list
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                for (idx, value) in values.iter().enumerate() {
                    self.set_variable(var_name.clone(), value.clone());
                    self.set_variable("_idx".to_string(), idx.to_string());
                    for stmt in body {
                        let signal = self.execute_statement(stmt, execute_command).await?;
                        match signal {
                            FlowSignal::Break(n) => {
                                if n <= 1 {
                                    return Ok(FlowSignal::None);
                                }
                                return Ok(FlowSignal::Break(n - 1));
                            }
                            FlowSignal::Continue(n) => {
                                if n <= 1 {
                                    break; // continue to next iteration
                                }
                                return Ok(FlowSignal::Continue(n - 1));
                            }
                            FlowSignal::Return(v) => return Ok(FlowSignal::Return(v)),
                            FlowSignal::None => {}
                        }
                    }
                }
                Ok(FlowSignal::None)
            }
            Statement::While(condition, body) => {
                let mut iterations = 0u64;
                while self.evaluate_condition(condition)? {
                    iterations += 1;
                    if iterations > 100_000 {
                        eprintln!("Warning: while loop exceeded 100000 iterations, breaking");
                        break;
                    }
                    for stmt in body {
                        let signal = self.execute_statement(stmt, execute_command).await?;
                        match signal {
                            FlowSignal::Break(n) => {
                                if n <= 1 {
                                    return Ok(FlowSignal::None);
                                }
                                return Ok(FlowSignal::Break(n - 1));
                            }
                            FlowSignal::Continue(n) => {
                                if n <= 1 {
                                    break; // continue to next iteration
                                }
                                return Ok(FlowSignal::Continue(n - 1));
                            }
                            FlowSignal::Return(v) => return Ok(FlowSignal::Return(v)),
                            FlowSignal::None => {}
                        }
                    }
                }
                Ok(FlowSignal::None)
            }
            Statement::FnDef(name, params, body) => {
                self.functions.insert(
                    name.clone(),
                    Function {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
                Ok(FlowSignal::None)
            }
        }
        })
    }

    /// Execute a function call
    fn execute_function_call<'a, F, Fut>(
        &'a mut self,
        name: &'a str,
        args: &'a [String],
        execute_command: &'a mut F,
    ) -> Pin<Box<dyn Future<Output = Result<FlowSignal>> + 'a>>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        Box::pin(async move {
            let func = match self.functions.get(name) {
                Some(f) => f.clone(),
                None => {
                    eprintln!("Error: function '{}' not defined", name);
                    return Ok(FlowSignal::None);
                }
            };

        // Save current param state and set function params
        let mut saved_vars = Vec::new();
        for (idx, param) in func.params.iter().enumerate() {
            let old_val = self.get_variable(param);
            saved_vars.push((param.clone(), old_val));
            let arg_val = args.get(idx).cloned().unwrap_or_default();
            self.set_variable(param.clone(), arg_val);
        }
        // Also set $1, $2, etc and $#
        let saved_positional: Vec<(String, Option<String>)> = (1..=args.len())
            .map(|i| {
                let key = i.to_string();
                let old = self.get_variable(&key);
                (key.clone(), old)
            })
            .collect();
        for (idx, arg) in args.iter().enumerate() {
            self.set_variable((idx + 1).to_string(), arg.clone());
        }
        let saved_argc = self.get_variable("#");
        self.set_variable("#".to_string(), args.len().to_string());

        let mut return_signal = FlowSignal::None;
        for stmt in &func.body {
            let signal = self.execute_statement(stmt, execute_command).await?;
            match signal {
                FlowSignal::Return(v) => {
                    if let Some(val) = v {
                        self.set_variable("?".to_string(), val);
                    }
                    return_signal = FlowSignal::None;
                    break;
                }
                FlowSignal::Break(_) | FlowSignal::Continue(_) => {
                    return_signal = signal;
                    break;
                }
                FlowSignal::None => {}
            }
        }

        // Restore params
        for (param, old_val) in saved_vars {
            match old_val {
                Some(v) => self.set_variable(param, v),
                None => self.remove_variable(&param),
            }
        }
        for (key, old_val) in saved_positional {
            match old_val {
                Some(v) => self.set_variable(key, v),
                None => self.remove_variable(&key),
            }
        }
        match saved_argc {
            Some(v) => self.set_variable("#".to_string(), v),
            None => self.remove_variable("#"),
        }

        Ok(return_signal)
        })
    }

    /// 评估条件表达式
    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        let condition = self.expand_variables(condition.trim());
        let condition = condition.trim();

        if condition.is_empty() {
            return Ok(false);
        }

        // 文件测试: -f file, -d dir, -e path, -z str, -n str
        if let Some(rest) = condition.strip_prefix("-f ") {
            return Ok(std::path::Path::new(rest.trim()).is_file());
        }
        if let Some(rest) = condition.strip_prefix("-d ") {
            return Ok(std::path::Path::new(rest.trim()).is_dir());
        }
        if let Some(rest) = condition.strip_prefix("-e ") {
            return Ok(std::path::Path::new(rest.trim()).exists());
        }
        if let Some(rest) = condition.strip_prefix("-z ") {
            return Ok(rest.trim().is_empty());
        }
        if let Some(rest) = condition.strip_prefix("-n ") {
            return Ok(!rest.trim().is_empty());
        }

        // String comparisons (check before numeric to avoid false splits)
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

        // Numeric comparisons (check multi-char operators first)
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
        match condition {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            s => Ok(!s.is_empty()),
        }
    }
}

/// 算术运算 token
#[derive(Debug, Clone)]
enum ArithToken {
    Num(f64),
    Plus,
    Minus,
    Mul,
    Div,
    Mod,
    Power,
    LParen,
    RParen,
}

/// 语句类型
#[derive(Debug, Clone)]
enum Statement {
    Command(String),
    If(String, Vec<Statement>, Option<Vec<Statement>>),
    For(String, String, Vec<Statement>),
    While(String, Vec<Statement>),
    FnDef(String, Vec<String>, Vec<Statement>),
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
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
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
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo 1", "echo 2", "echo 3"]);
    }

    #[tokio::test]
    async fn test_variable_expansion() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("NAME".to_string(), "world".to_string());
        assert_eq!(executor.expand_variables("echo $NAME"), "echo world");
        assert_eq!(executor.expand_variables("echo ${NAME}"), "echo world");
    }

    #[tokio::test]
    async fn test_if_else() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
if "a" == "b" {
    echo nope
} else {
    echo yes
}
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo yes"]);
    }

    #[tokio::test]
    async fn test_nested_for() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
for x in a b {
    for y in 1 2 {
        echo $x $y
    }
}
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo a 1", "echo a 2", "echo b 1", "echo b 2"]);
    }

    #[tokio::test]
    async fn test_variable_in_for_loop() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("ITEMS".to_string(), "x y z".to_string());
        let expanded = executor.expand_variables("for i in $ITEMS { echo $i }");
        assert!(expanded.contains("x y z"), "Variable should expand in for loop");
    }

    // ── New feature tests ────────────────────────────────────

    #[tokio::test]
    async fn test_function_definition_and_call() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
fn greet(name) {
    echo hello $name
}
greet world
greet rust
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo hello world", "echo hello rust"]);
    }

    #[tokio::test]
    async fn test_function_with_return() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
fn add(a, b) {
    echo result is $((a + b))
}
add 3 4
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo result is 7"]);
    }

    #[test]
    fn test_arithmetic_addition() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$((3 + 4))"), "7");
    }

    #[test]
    fn test_arithmetic_multiplication() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$((6 * 7))"), "42");
    }

    #[test]
    fn test_arithmetic_power() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$((2 ** 10))"), "1024");
    }

    #[test]
    fn test_arithmetic_modulo() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$((17 % 5))"), "2");
    }

    #[test]
    fn test_arithmetic_parentheses() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$(((2 + 3) * 4))"), "20");
    }

    #[test]
    fn test_arithmetic_with_variables() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("x".to_string(), "10".to_string());
        executor.set_variable("y".to_string(), "5".to_string());
        assert_eq!(executor.expand_variables("$((x + y))"), "15");
        assert_eq!(executor.expand_variables("$((x * y))"), "50");
    }

    #[test]
    fn test_string_length() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("NAME".to_string(), "hello".to_string());
        assert_eq!(executor.expand_variables("${#NAME}"), "5");
    }

    #[test]
    fn test_default_value() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("${UNSET_VAR:-fallback}"), "fallback");
    }

    #[test]
    fn test_default_value_set() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("SET_VAR".to_string(), "actual".to_string());
        assert_eq!(executor.expand_variables("${SET_VAR:-fallback}"), "actual");
    }

    #[test]
    fn test_substring() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("STR".to_string(), "hello world".to_string());
        assert_eq!(executor.expand_variables("${STR:0:5}"), "hello");
        assert_eq!(executor.expand_variables("${STR:6:5}"), "world");
    }

    #[test]
    fn test_alternative_value() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("X".to_string(), "yes".to_string());
        assert_eq!(executor.expand_variables("${X:+set}"), "set");
        assert_eq!(executor.expand_variables("${EMPTY:+set}"), "");
    }

    #[tokio::test]
    async fn test_break_in_for() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
for i in 1 2 3 4 5 {
    if $i == 3 {
        break
    }
    echo $i
}
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo 1", "echo 2"]);
    }

    #[tokio::test]
    async fn test_continue_in_for() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
for i in 1 2 3 4 5 {
    if $i == 3 {
        continue
    }
    echo $i
}
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo 1", "echo 2", "echo 4", "echo 5"]);
    }

    #[tokio::test]
    async fn test_while_with_break() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("n".to_string(), "0".to_string());
        let script = r#"
while 1 == 1 {
    echo $n
    n=$((n + 1))
    if $n == 3 {
        break
    }
}
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert!(commands.contains(&"echo 0".to_string()));
    }

    #[test]
    fn test_arithmetic_complex() {
        let executor = ControlFlowExecutor::new();
        assert_eq!(executor.expand_variables("$((100 / 3))"), "33");
        assert_eq!(executor.expand_variables("$((2 ** 8 - 1))"), "255");
        assert_eq!(executor.expand_variables("$((10 - -5))"), "15");
    }

    #[tokio::test]
    async fn test_function_no_params() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
fn hello() {
    echo world
}
hello
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo world"]);
    }

    #[tokio::test]
    async fn test_function_scopes_vars() {
        let mut executor = ControlFlowExecutor::new();
        let script = r#"
fn test_scope(x) {
    echo inner $x
}
test_scope 42
echo outer
"#;
        let mut commands = Vec::new();
        executor.execute_script(script, |cmd| { commands.push(cmd); async { Ok(()) } }).await.unwrap();
        assert_eq!(commands, vec!["echo inner 42", "echo outer"]);
    }

    #[test]
    fn test_string_offset() {
        let mut executor = ControlFlowExecutor::new();
        executor.set_variable("S".to_string(), "abcdef".to_string());
        assert_eq!(executor.expand_variables("${S:3}"), "def");
    }
}
