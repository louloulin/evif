use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, PluginConfigParam, WriteFlags,
};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::{OnceCell, RwLock};

use crate::{skill_runtime, MemFsPlugin};
pub use skill_runtime::{SkillExecutor, SkillExecutionContext, SkillExecutionResult};

// ---------------------------------------------------------------------------
// SKILL.md validation types (Phase 9.1 SkillFS – inline agent-skills compat)
// ---------------------------------------------------------------------------

/// Validated metadata extracted from a SKILL.md file.
///
/// This struct is returned by [`validate_skill_md`] and represents a fully
/// validated SKILL.md document where the YAML frontmatter contains the
/// required fields (`name`, `description`, `triggers`) and the Markdown body
/// is non-empty.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillMetadata {
    /// Skill identifier. Must be non-empty and contain only lowercase
    /// alphanumeric characters, hyphens, and underscores.
    pub name: String,
    /// Human-readable description of what the skill does.
    pub description: String,
    /// Trigger phrases used for skill matching.
    pub triggers: Vec<String>,
    /// The Markdown body content (everything after the YAML frontmatter).
    pub body: String,
}

/// Errors that can occur during SKILL.md validation.
#[derive(Clone, Debug, PartialEq)]
pub enum SkillValidationError {
    /// The document contains no YAML frontmatter delimiters (`---`).
    MissingFrontmatter,
    /// The YAML frontmatter could not be parsed.
    FrontmatterParseError(String),
    /// A required frontmatter field is missing or empty.
    MissingRequiredField { field: String, reason: String },
    /// The skill name contains invalid characters.
    InvalidName { name: String, reason: String },
    /// The Markdown body (after frontmatter) is empty.
    EmptyBody,
    /// A trigger entry is empty (blank string).
    EmptyTrigger(usize),
}

impl fmt::Display for SkillValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFrontmatter => {
                write!(f, "SKILL.md is missing YAML frontmatter (expected `---` delimiters)")
            }
            Self::FrontmatterParseError(msg) => {
                write!(f, "SKILL.md frontmatter parse error: {}", msg)
            }
            Self::MissingRequiredField { field, reason } => {
                write!(
                    f,
                    "SKILL.md missing required field '{}': {}",
                    field, reason
                )
            }
            Self::InvalidName { name, reason } => {
                write!(f, "SKILL.md invalid name '{}': {}", name, reason)
            }
            Self::EmptyBody => {
                write!(f, "SKILL.md body is empty (content after frontmatter is required)")
            }
            Self::EmptyTrigger(index) => {
                write!(
                    f,
                    "SKILL.md trigger at index {} is empty",
                    index
                )
            }
        }
    }
}

impl std::error::Error for SkillValidationError {}

/// Validate a raw SKILL.md string and return the extracted [`SkillMetadata`].
///
/// # Validation rules
///
/// 1. **Frontmatter present**: The content must contain YAML frontmatter
///    delimited by `---`.
/// 2. **`name` required**: Must be non-empty. Only lowercase alphanumeric
///    characters, hyphens (`-`), and underscores (`_`) are allowed.
/// 3. **`description` required**: Must be non-empty.
/// 4. **`triggers` required**: Must contain at least one entry. Each trigger
///    must be a non-empty string.
/// 5. **Body non-empty**: The Markdown content after the frontmatter must not
///    be blank.
///
/// # Errors
///
/// Returns [`SkillValidationError`] when any validation rule is violated.
///
/// # Example
///
/// ```
/// use evif_plugins::skillfs::{validate_skill_md, SkillMetadata};
///
/// let content = r#"---
/// name: my-skill
/// description: "A test skill"
/// triggers:
///   - "do thing"
/// ---
/// # My Skill
/// Do the thing.
/// "#;
///
/// let meta = validate_skill_md(content).unwrap();
/// assert_eq!(meta.name, "my-skill");
/// ```
pub fn validate_skill_md(content: &str) -> Result<SkillMetadata, SkillValidationError> {
    let matter = Matter::<YAML>::new();
    let result = matter
        .parse::<SkillFrontmatter>(content)
        .map_err(|err| {
            let msg = err.to_string();
            // Map serde "missing field" errors to our structured error type
            if msg.contains("missing field `name`") {
                SkillValidationError::MissingRequiredField {
                    field: "name".to_string(),
                    reason: "field is missing from YAML frontmatter".to_string(),
                }
            } else if msg.contains("missing field `description`") {
                SkillValidationError::MissingRequiredField {
                    field: "description".to_string(),
                    reason: "field is missing from YAML frontmatter".to_string(),
                }
            } else if msg.contains("missing field `triggers`") {
                SkillValidationError::MissingRequiredField {
                    field: "triggers".to_string(),
                    reason: "field is missing from YAML frontmatter".to_string(),
                }
            } else {
                SkillValidationError::FrontmatterParseError(msg)
            }
        })?;

    let parsed = result
        .data
        .ok_or(SkillValidationError::MissingFrontmatter)?;

    // 1. Validate `name`
    let name = parsed.name.trim().to_string();
    if name.is_empty() {
        return Err(SkillValidationError::MissingRequiredField {
            field: "name".to_string(),
            reason: "field is empty or missing".to_string(),
        });
    }
    if !is_valid_skill_name(&name) {
        return Err(SkillValidationError::InvalidName {
            name: name.clone(),
            reason: "name must contain only lowercase alphanumeric characters, hyphens, and underscores"
                .to_string(),
        });
    }

    // 2. Validate `description`
    let description = parsed.description.trim().to_string();
    if description.is_empty() {
        return Err(SkillValidationError::MissingRequiredField {
            field: "description".to_string(),
            reason: "field is empty or missing".to_string(),
        });
    }

    // 3. Validate `triggers`
    if parsed.triggers.is_empty() {
        return Err(SkillValidationError::MissingRequiredField {
            field: "triggers".to_string(),
            reason: "at least one trigger is required".to_string(),
        });
    }
    for (i, trigger) in parsed.triggers.iter().enumerate() {
        if trigger.trim().is_empty() {
            return Err(SkillValidationError::EmptyTrigger(i));
        }
    }
    let triggers: Vec<String> = parsed
        .triggers
        .iter()
        .map(|t| t.trim().to_string())
        .collect();

    // 4. Validate body is non-empty
    let body = result.content.trim().to_string();
    if body.is_empty() {
        return Err(SkillValidationError::EmptyBody);
    }

    Ok(SkillMetadata {
        name,
        description,
        triggers,
        body,
    })
}

/// Check whether a skill name contains only valid characters:
/// lowercase a-z, digits 0-9, hyphens, and underscores.
fn is_valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct SkillDefinition {
    name: String,
    description: String,
    triggers: Vec<String>,
    body: String,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    triggers: Vec<String>,
}

pub struct SkillFsPlugin {
    inner: MemFsPlugin,
    initialized: OnceCell<()>,
    definitions: RwLock<HashMap<String, SkillDefinition>>,
    executor: SkillExecutor,
    execution_context: SkillExecutionContext,
}

impl SkillFsPlugin {
    pub fn new() -> Self {
        Self {
            inner: MemFsPlugin::new(),
            initialized: OnceCell::const_new(),
            definitions: RwLock::new(HashMap::new()),
            executor: SkillExecutor::Native,
            execution_context: SkillExecutionContext::new(),
        }
    }

    pub fn with_executor(mut self, executor: SkillExecutor) -> Self {
        self.executor = executor;
        self
    }

    /// Execute a skill using the configured executor.
    /// Returns the raw JSON string result from the skill runtime.
    pub async fn execute_skill_raw(&self, name: &str, input: &str) -> Result<String, skill_runtime::SkillRuntimeError> {
        skill_runtime::execute_skill(name, input, self.executor, &self.execution_context).await
    }

    pub async fn match_skill(&self, query: &str) -> EvifResult<Option<String>> {
        self.ensure_initialized().await?;

        let lowered = query.to_ascii_lowercase();
        let definitions = self.definitions.read().await;
        for definition in definitions.values() {
            if definition
                .triggers
                .iter()
                .any(|trigger| lowered.contains(&trigger.to_ascii_lowercase()))
            {
                return Ok(Some(definition.name.clone()));
            }
        }

        Ok(None)
    }

    fn readme_text(&self) -> String {
        r#"# SkillFS

标准 `SKILL.md` 技能文件系统。

## 当前实现

- 预置内建技能目录与标准 `SKILL.md`
- 支持通过 `ls` / `cat` 发现技能
- 支持写入 `input` 并将结果写入 `output`
- 支持按 `triggers` 做最小自然语言匹配

## 说明

这是最小可用版本，尚未接入 MCP 暴露与安全执行沙箱。
"#
        .to_string()
    }

    fn builtin_skill_docs() -> Vec<String> {
        vec![
            r#"---
name: code-review
description: "Review code for bugs, security issues, and maintainability"
triggers:
  - "review"
  - "code review"
  - "check my code"
---

# Code Review

Read the target code, identify the most important risks, and produce a concise review report.
"#
            .to_string(),
            r#"---
name: test-gen
description: "Generate focused regression tests for the requested code path"
triggers:
  - "test"
  - "generate test"
  - "regression"
---

# Test Generation

Produce minimal automated tests that capture the requested behavior.
"#
            .to_string(),
            r#"---
name: doc-gen
description: "Generate concise documentation for code, APIs, or workflows"
triggers:
  - "docs"
  - "document"
  - "write docs"
---

# Documentation Generation

Write concise developer-facing documentation from the provided inputs.
"#
            .to_string(),
            r#"---
name: refactor
description: "Suggest and apply focused refactors without changing behavior"
triggers:
  - "refactor"
  - "clean up"
  - "simplify"
---

# Refactor

Identify safe refactor opportunities and describe the intended structural change.
"#
            .to_string(),
        ]
    }

    async fn ensure_initialized(&self) -> EvifResult<()> {
        self.initialized
            .get_or_try_init(|| async {
                self.seed_file("/README", self.readme_text()).await?;

                for raw in Self::builtin_skill_docs() {
                    self.register_skill_document(&raw).await?;
                }

                Ok::<(), EvifError>(())
            })
            .await?;
        Ok(())
    }

    async fn ensure_skill_channels(&self, skill_name: &str) -> EvifResult<()> {
        let skill_path = format!("/{}", skill_name);
        if self.inner.stat(&skill_path).await.is_err() {
            self.inner.mkdir(&skill_path, 0o755).await?;
        }

        for channel in ["input", "output"] {
            let path = format!("{}/{}", skill_path, channel);
            if self.inner.stat(&path).await.is_err() {
                self.inner.create(&path, 0o644).await?;
            }
        }

        Ok(())
    }

    async fn overwrite_file(&self, path: &str, data: &[u8]) -> EvifResult<()> {
        if self.inner.stat(path).await.is_err() {
            self.inner.create(path, 0o644).await?;
        }

        self.inner
            .write(path, data.to_vec(), 0, WriteFlags::TRUNCATE)
            .await?;
        Ok(())
    }

    async fn seed_file(&self, path: &str, content: String) -> EvifResult<()> {
        self.overwrite_file(path, content.as_bytes()).await
    }

    async fn register_skill_document(&self, raw: &str) -> EvifResult<()> {
        let definition = Self::parse_skill_document(raw)?;
        let skill_name = definition.name.clone();
        self.ensure_skill_channels(&skill_name).await?;
        self.overwrite_file(&format!("/{}/SKILL.md", skill_name), raw.as_bytes())
            .await?;
        self.definitions.write().await.insert(skill_name, definition);
        Ok(())
    }

    fn parse_skill_document(raw: &str) -> EvifResult<SkillDefinition> {
        let metadata = validate_skill_md(raw).map_err(|err| {
            EvifError::Deserialization(format!("SKILL.md validation failed: {}", err))
        })?;

        Ok(SkillDefinition {
            name: metadata.name,
            description: metadata.description,
            triggers: metadata.triggers,
            body: metadata.body,
        })
    }

    pub async fn list_skill_definitions(&self) -> Vec<(String, String, Vec<String>)> {
        self.ensure_initialized().await.ok();
        let definitions = self.definitions.read().await;
        definitions
            .values()
            .map(|d| (d.name.clone(), d.description.clone(), d.triggers.clone()))
            .collect()
    }

    pub async fn export_claude_skills_dir(&self, output_dir: &str) -> EvifResult<Vec<String>> {
        self.ensure_initialized().await?;

        let definitions = self.definitions.read().await;
        let mut exported = Vec::new();

        for definition in definitions.values() {
            let skill_dir = format!("{}/{}", output_dir, definition.name);
            tokio::fs::create_dir_all(&skill_dir).await?;

            let raw_content = self
                .inner
                .read(&format!("/{}/SKILL.md", definition.name), 0, 0)
                .await?;
            let content_str = String::from_utf8(raw_content)
                .map_err(|err| EvifError::InvalidInput(err.to_string()))?;

            let skill_md_path = format!("{}/SKILL.md", skill_dir);
            tokio::fs::write(&skill_md_path, content_str.as_bytes()).await?;

            exported.push(definition.name.clone());
        }

        Ok(exported)
    }

    pub async fn export_codex_agents_yaml(&self, output_path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;

        let definitions = self.definitions.read().await;

        #[derive(Serialize)]
        struct CodexSkillEntry {
            name: String,
            description: String,
            triggers: Vec<String>,
        }

        #[derive(Serialize)]
        struct CodexAgentsYaml {
            version: String,
            generated_by: String,
            skills: Vec<CodexSkillEntry>,
        }

        let codex_yaml = CodexAgentsYaml {
            version: "1.0".to_string(),
            generated_by: "evif-skillfs".to_string(),
            skills: definitions
                .values()
                .map(|d| CodexSkillEntry {
                    name: d.name.clone(),
                    description: d.description.clone(),
                    triggers: d.triggers.clone(),
                })
                .collect(),
        };

        if let Some(parent) = std::path::Path::new(output_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let yaml_str = serde_yaml::to_string(&codex_yaml)
            .map_err(|err| EvifError::Serialization(err.to_string()))?;
        tokio::fs::write(output_path, yaml_str).await?;

        Ok(())
    }

    pub async fn import_claude_skills_dir(&self, input_dir: &str) -> EvifResult<Vec<String>> {
        self.ensure_initialized().await?;

        let mut imported = Vec::new();
        let mut entries = tokio::fs::read_dir(input_dir)
            .await
            .map_err(EvifError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(EvifError::Io)? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_md_path = path.join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            let raw_content = tokio::fs::read_to_string(&skill_md_path).await?;
            self.register_skill_document(&raw_content).await?;

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                imported.push(name.to_string());
            }
        }

        Ok(imported)
    }

    /// Generate a Codex-compatible `agents/openai.yaml` string from all discovered skills.
    ///
    /// Returns the YAML as a String, suitable for writing to `agents/openai.yaml`
    /// or passing to any caller that manages the filesystem.
    ///
    /// The output format is:
    /// ```yaml
    /// agents:
    ///   - name: skill-name
    ///     description: "skill description"
    ///     triggers:
    ///       - "trigger phrase"
    ///     instructions: |
    ///       skill body content here
    /// ```
    pub async fn generate_openai_yaml(&self) -> EvifResult<String> {
        self.ensure_initialized().await?;

        let definitions = self.definitions.read().await;

        #[derive(Serialize)]
        struct CodexAgentEntry {
            name: String,
            description: String,
            triggers: Vec<String>,
            instructions: String,
        }

        #[derive(Serialize)]
        struct CodexAgentsFile {
            agents: Vec<CodexAgentEntry>,
        }

        let agents_file = CodexAgentsFile {
            agents: definitions
                .values()
                .map(|d| CodexAgentEntry {
                    name: d.name.clone(),
                    description: d.description.clone(),
                    triggers: d.triggers.clone(),
                    instructions: d.body.clone(),
                })
                .collect(),
        };

        let yaml_str = serde_yaml::to_string(&agents_file)
            .map_err(|err| EvifError::Serialization(err.to_string()))?;

        Ok(yaml_str)
    }

    /// Return a mapping of all skills to Claude Code's `.claude/skills/` format.
    ///
    /// Each entry is a `(relative_path, content)` tuple where:
    /// - `relative_path` is the path relative to `.claude/skills/`, e.g.
    ///   `code-review/SKILL.md`
    /// - `content` is the raw SKILL.md file content (YAML frontmatter + body)
    ///
    /// The caller can write these files or create symlinks as needed.
    pub async fn sync_to_claude_skills(&self) -> EvifResult<Vec<(String, String)>> {
        self.ensure_initialized().await?;

        let definitions = self.definitions.read().await;
        let mut result = Vec::with_capacity(definitions.len());

        for definition in definitions.values() {
            let raw_content = self
                .inner
                .read(&format!("/{}/SKILL.md", definition.name), 0, 0)
                .await?;
            let content_str = String::from_utf8(raw_content)
                .map_err(|err| EvifError::InvalidInput(err.to_string()))?;

            let relative_path = format!("{}/SKILL.md", definition.name);
            result.push((relative_path, content_str));
        }

        Ok(result)
    }

    fn split_path(path: &str) -> Vec<&str> {
        path.trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }
}

impl Default for SkillFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for SkillFsPlugin {
    fn name(&self) -> &str {
        "skillfs"
    }

    fn get_readme(&self) -> String {
        self.readme_text()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![]
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.create(path, perm).await
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.ensure_initialized().await?;

        let parts = Self::split_path(path);
        if parts.len() == 1 {
            self.inner.mkdir(path, perm).await?;
            self.ensure_skill_channels(parts[0]).await?;
            return Ok(());
        }

        self.inner.mkdir(path, perm).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        self.ensure_initialized().await?;
        self.inner.read(path, offset, size).await
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64> {
        self.ensure_initialized().await?;

        let parts = Self::split_path(path);
        if parts.len() == 2 && parts[1] == "SKILL.md" {
            let raw = String::from_utf8(data.clone())
                .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
            self.register_skill_document(&raw).await?;
            return Ok(data.len() as u64);
        }

        if parts.len() == 2 && parts[1] == "input" {
            let skill_name = parts[0];
            self.ensure_skill_channels(skill_name).await?;
            let bytes = self.inner.write(path, data.clone(), offset, flags).await?;

            // Execute the skill using the configured executor
            let input_str = String::from_utf8_lossy(&data).to_string();
            match self.execute_skill_raw(skill_name, &input_str).await {
                Ok(result_json) => {
                    // execute_skill_raw returns JSON string directly
                    self.overwrite_file(
                        &format!("/{}/output", skill_name),
                        result_json.as_bytes(),
                    )
                    .await?;
                }
                Err(e) => {
                    // On error, store error in output
                    let error_output = serde_json::json!({
                        "skill": skill_name,
                        "status": "error",
                        "error": e.to_string(),
                        "mode": self.executor.to_string()
                    });
                    self.overwrite_file(
                        &format!("/{}/output", skill_name),
                        error_output.to_string().as_bytes(),
                    )
                    .await?;
                }
            }
            return Ok(bytes);
        }

        self.inner.write(path, data, offset, flags).await
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.ensure_initialized().await?;
        self.inner.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.ensure_initialized().await?;
        self.inner.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        let parts = Self::split_path(path);
        if parts.len() == 1 {
            self.definitions.write().await.remove(parts[0]);
        }
        self.inner.remove(path).await
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.rename(old_path, new_path).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.remove_all(path).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_skill_md: valid documents --------------------------------

    #[test]
    fn validate_valid_skill_md() {
        let content = r#"---
name: code-review
description: "Review code for bugs and security issues"
triggers:
  - "review"
  - "code review"
  - "check my code"
---

# Code Review

Read the target code, identify the most important risks, and produce a concise review report.
"#;
        let meta = validate_skill_md(content).unwrap();
        assert_eq!(meta.name, "code-review");
        assert_eq!(
            meta.description,
            "Review code for bugs and security issues"
        );
        assert_eq!(meta.triggers, vec!["review", "code review", "check my code"]);
        assert!(meta.body.contains("# Code Review"));
    }

    #[test]
    fn validate_skill_with_underscore_name() {
        let content = r#"---
name: my_skill_123
description: "A skill with underscores"
triggers:
  - "trigger"
---

Body content here.
"#;
        let meta = validate_skill_md(content).unwrap();
        assert_eq!(meta.name, "my_skill_123");
    }

    #[test]
    fn validate_skill_with_minimal_body() {
        let content = r#"---
name: minimal
description: "Minimal skill"
triggers:
  - "go"
---

x
"#;
        let meta = validate_skill_md(content).unwrap();
        assert_eq!(meta.body, "x");
    }

    // -- validate_skill_md: missing frontmatter ----------------------------

    #[test]
    fn validate_rejects_missing_frontmatter() {
        let content = "Just some plain text with no frontmatter at all.";
        let err = validate_skill_md(content).unwrap_err();
        assert_eq!(err, SkillValidationError::MissingFrontmatter);
    }

    #[test]
    fn validate_rejects_empty_string() {
        let err = validate_skill_md("").unwrap_err();
        assert_eq!(err, SkillValidationError::MissingFrontmatter);
    }

    // -- validate_skill_md: missing required fields -------------------------

    #[test]
    fn validate_rejects_missing_name() {
        let content = r#"---
description: "Has description"
triggers:
  - "something"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, .. } => {
                assert_eq!(field, "name");
            }
            other => panic!("expected MissingRequiredField for name, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_empty_name() {
        let content = r#"---
name: ""
description: "Has description"
triggers:
  - "something"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, .. } => {
                assert_eq!(field, "name");
            }
            other => panic!("expected MissingRequiredField for name, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_missing_description() {
        let content = r#"---
name: my-skill
triggers:
  - "something"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, .. } => {
                assert_eq!(field, "description");
            }
            other => panic!("expected MissingRequiredField for description, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_empty_description() {
        let content = r#"---
name: my-skill
description: ""
triggers:
  - "something"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, .. } => {
                assert_eq!(field, "description");
            }
            other => panic!("expected MissingRequiredField for description, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_missing_triggers() {
        let content = r#"---
name: my-skill
description: "A skill"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, .. } => {
                assert_eq!(field, "triggers");
            }
            other => panic!("expected MissingRequiredField for triggers, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_empty_triggers_list() {
        let content = r#"---
name: my-skill
description: "A skill"
triggers: []
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::MissingRequiredField { field, reason } => {
                assert_eq!(field, "triggers");
                assert!(reason.contains("at least one"));
            }
            other => panic!("expected MissingRequiredField for triggers, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_empty_trigger_entry() {
        let content = r#"---
name: my-skill
description: "A skill"
triggers:
  - "valid"
  - ""
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        assert_eq!(err, SkillValidationError::EmptyTrigger(1));
    }

    // -- validate_skill_md: invalid name -----------------------------------

    #[test]
    fn validate_rejects_uppercase_name() {
        let content = r#"---
name: My-Skill
description: "A skill"
triggers:
  - "trigger"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        match err {
            SkillValidationError::InvalidName { name, .. } => {
                assert_eq!(name, "My-Skill");
            }
            other => panic!("expected InvalidName, got: {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_name_with_spaces() {
        let content = r#"---
name: my skill
description: "A skill"
triggers:
  - "trigger"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        assert!(matches!(err, SkillValidationError::InvalidName { .. }));
    }

    #[test]
    fn validate_rejects_name_with_special_chars() {
        let content = r#"---
name: my@skill!
description: "A skill"
triggers:
  - "trigger"
---

Body here.
"#;
        let err = validate_skill_md(content).unwrap_err();
        assert!(matches!(err, SkillValidationError::InvalidName { .. }));
    }

    // -- validate_skill_md: empty body -------------------------------------

    #[test]
    fn validate_rejects_empty_body() {
        let content = r#"---
name: my-skill
description: "A skill"
triggers:
  - "trigger"
---

"#;
        let err = validate_skill_md(content).unwrap_err();
        assert_eq!(err, SkillValidationError::EmptyBody);
    }

    #[test]
    fn validate_rejects_whitespace_only_body() {
        let content = r#"---
name: my-skill
description: "A skill"
triggers:
  - "trigger"
---



"#;
        let err = validate_skill_md(content).unwrap_err();
        assert_eq!(err, SkillValidationError::EmptyBody);
    }

    // -- is_valid_skill_name ------------------------------------------------

    #[test]
    fn test_is_valid_skill_name() {
        assert!(is_valid_skill_name("code-review"));
        assert!(is_valid_skill_name("my_skill"));
        assert!(is_valid_skill_name("skill123"));
        assert!(is_valid_skill_name("a"));
        assert!(is_valid_skill_name("a-b_c"));
        assert!(!is_valid_skill_name("My-Skill"));
        assert!(!is_valid_skill_name("my skill"));
        assert!(!is_valid_skill_name(""));
        assert!(!is_valid_skill_name("skill@name"));
        assert!(!is_valid_skill_name("skill.name"));
    }

    // -- SkillValidationError display --------------------------------------

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            SkillValidationError::MissingFrontmatter.to_string(),
            "SKILL.md is missing YAML frontmatter (expected `---` delimiters)"
        );
        assert_eq!(
            SkillValidationError::EmptyBody.to_string(),
            "SKILL.md body is empty (content after frontmatter is required)"
        );
        assert_eq!(
            SkillValidationError::EmptyTrigger(2).to_string(),
            "SKILL.md trigger at index 2 is empty"
        );
    }

    // -- parse_skill_document integration -----------------------------------

    #[test]
    fn parse_skill_document_valid() {
        let content = r#"---
name: test-skill
description: "A test skill"
triggers:
  - "test"
---

# Test Skill

This is the body.
"#;
        let def = SkillFsPlugin::parse_skill_document(content).unwrap();
        assert_eq!(def.name, "test-skill");
        assert_eq!(def.description, "A test skill");
        assert_eq!(def.triggers, vec!["test"]);
        assert!(def.body.contains("# Test Skill"));
    }

    #[test]
    fn parse_skill_document_invalid_name() {
        let content = r#"---
name: "INVALID"
description: "A skill"
triggers:
  - "trigger"
---

Body.
"#;
        let err = SkillFsPlugin::parse_skill_document(content).unwrap_err();
        assert!(err.to_string().contains("SKILL.md validation failed"));
        assert!(err.to_string().contains("invalid name"));
    }

    #[test]
    fn parse_skill_document_missing_triggers() {
        let content = r#"---
name: valid-name
description: "A skill"
---

Body.
"#;
        let err = SkillFsPlugin::parse_skill_document(content).unwrap_err();
        assert!(err.to_string().contains("SKILL.md validation failed"));
        assert!(err.to_string().contains("triggers"));
    }

    // -- builtin skill docs remain valid after validation change -----------

    #[test]
    fn builtin_skill_docs_are_valid() {
        for doc in SkillFsPlugin::builtin_skill_docs() {
            let meta = validate_skill_md(&doc);
            assert!(meta.is_ok(), "builtin skill doc failed validation: {:?}", meta.err());
        }
    }
}
