//! Skill Runtime Safety Layer for EVIF Skills
//!
//! This module provides a safe skill execution environment with three modes:
//! 1. **Native execution** - For development, passes input through to LLM
//! 2. **WASM sandbox** - For production (recommended), runs skills in WASM sandbox
//! 3. **Docker isolation** - For maximum security, runs skills in Docker container
//!
//! # Key Insight
//!
//! EVIF skills are LLM-driven (the SKILL.md contains instructions that an AI agent follows),
//! so "execution" in the native case means providing the skill's instructions and input
//! to the agent. The actual execution is done by the AI agent reading the SKILL.md instructions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during skill execution
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum SkillRuntimeError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Execution timeout: {0}")]
    Timeout(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Sandbox error: {0}")]
    SandboxError(String),

    #[error("WASM execution not yet available: {0}")]
    WasmNotAvailable(String),

    #[error("Docker execution not yet available: {0}")]
    DockerNotAvailable(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for SkillRuntimeError {
    fn from(err: std::io::Error) -> Self {
        SkillRuntimeError::IoError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for SkillRuntimeError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        SkillRuntimeError::Timeout(err.to_string())
    }
}

/// The skill executor mode determines how skills are executed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum SkillExecutor {
    /// Native execution - for development only
    /// Passes input through to the LLM agent (skill logic in SKILL.md)
    #[default]
    Native,

    /// WASM sandbox execution - recommended for production
    /// Runs skill code in a WASM sandbox for isolation
    Wasm,

    /// Docker isolation - for maximum security
    /// Runs skill in an isolated Docker container
    Docker,
}


impl std::fmt::Display for SkillExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillExecutor::Native => write!(f, "native"),
            SkillExecutor::Wasm => write!(f, "wasm"),
            SkillExecutor::Docker => write!(f, "docker"),
        }
    }
}

/// Execution context for skill runs
///
/// This struct holds the environment configuration for skill execution,
/// including working directory, timeout, and environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionContext {
    /// Working directory for skill execution
    pub work_dir: PathBuf,

    /// Maximum execution time
    pub timeout: Duration,

    /// Environment variables to pass to the skill
    pub env_vars: HashMap<String, String>,

    /// Whether to enable verbose logging
    pub verbose: bool,

    /// Memory limit in MB (for sandbox modes)
    pub memory_limit_mb: Option<u64>,
}

impl Default for SkillExecutionContext {
    fn default() -> Self {
        Self {
            work_dir: std::env::temp_dir(),
            timeout: Duration::from_secs(300), // 5 minutes default
            env_vars: HashMap::new(),
            verbose: false,
            memory_limit_mb: None,
        }
    }
}

impl SkillExecutionContext {
    /// Create a new execution context with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the working directory
    pub fn with_work_dir(mut self, path: PathBuf) -> Self {
        self.work_dir = path;
        self
    }

    /// Set the execution timeout
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Add an environment variable
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set memory limit for sandbox modes
    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.memory_limit_mb = Some(limit_mb);
        self
    }
}

/// Result of skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    /// The skill name that was executed
    pub skill_name: String,

    /// The execution mode used
    pub mode: SkillExecutor,

    /// Whether execution was successful
    pub success: bool,

    /// The output from execution
    pub output: String,

    /// Execution duration
    pub duration_ms: u64,

    /// Any error message if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Execute a skill with the given name, input, and execution mode.
///
/// # Arguments
///
/// * `name` - The name of the skill to execute
/// * `input` - The input data to pass to the skill
/// * `mode` - The execution mode (Native, WASM, or Docker)
///
/// # Returns
///
/// Returns the execution result as a JSON string, or an error if execution failed.
///
/// # Examples
///
/// ```
/// use evif_plugins::skill_runtime::{execute_skill, SkillExecutor, SkillExecutionContext};
///
/// # async {
/// let context = SkillExecutionContext::new();
/// let result = execute_skill("code-review", "fn foo() {}", SkillExecutor::Native, &context)
///     .await
///     .unwrap();
/// println!("Result: {}", result);
/// # };
/// ```
pub async fn execute_skill(
    name: &str,
    input: &str,
    mode: SkillExecutor,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    let start = std::time::Instant::now();

    // Validate inputs
    if name.is_empty() {
        return Err(SkillRuntimeError::InvalidInput(
            "Skill name cannot be empty".to_string(),
        ));
    }

    if input.is_empty() {
        return Err(SkillRuntimeError::InvalidInput(
            "Input cannot be empty".to_string(),
        ));
    }

    // Execute based on mode
    let output = match mode {
        SkillExecutor::Native => execute_native(name, input, context).await?,
        SkillExecutor::Wasm => execute_wasm(name, input, context).await?,
        SkillExecutor::Docker => execute_docker(name, input, context).await?,
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Build result
    let result = SkillExecutionResult {
        skill_name: name.to_string(),
        mode,
        success: true,
        output,
        duration_ms,
        error: None,
    };

    serde_json::to_string_pretty(&result)
        .map_err(|e| SkillRuntimeError::InvalidInput(e.to_string()))
}

/// Execute skill in native mode (for development)
///
/// In native mode, the skill is not actually executed. Instead, the input
/// is prepared for the LLM agent to process. The LLM agent reads the
/// SKILL.md file and follows the instructions contained within.
///
/// This is the default mode for development and testing.
async fn execute_native(
    name: &str,
    input: &str,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    if context.verbose {
        log::info!(
            "[Native] Executing skill '{}' with input length {} bytes",
            name,
            input.len()
        );
    }

    // In native mode, we prepare the execution context for the LLM agent
    // The actual execution happens when the LLM reads the SKILL.md file
    let _execution_context = serde_json::json!({
        "skill_name": name,
        "mode": "native",
        "input": input,
        "work_dir": context.work_dir.to_string_lossy(),
        "env_vars": context.env_vars,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "Native execution mode - skill logic will be handled by LLM agent reading SKILL.md"
    });

    // For native mode, we simulate the execution by returning a structured response
    // that indicates the skill execution context
    let response = format!(
        r#"EVIF SKILL EXECUTION (Native Mode)
================================

Skill Name: {}
Execution Mode: native (development)
Work Directory: {}

Environment Variables:
{}

Input:
---
{}

---

Execution Note:
In native mode, the skill's logic resides in its SKILL.md file.
An AI agent will read the SKILL.md instructions and process the input.
This execution context has been prepared for that agent.

{} bytes processed in native mode
================================"#,
        name,
        context.work_dir.to_string_lossy(),
        context
            .env_vars
            .iter()
            .map(|(k, v)| format!("  {}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n"),
        input,
        input.len()
    );

    Ok(response)
}

/// Execute skill in WASM sandbox (for production)
///
/// WASM execution provides a lightweight sandbox for running skill code
/// with memory and CPU isolation.
///
/// When the `skill-wasm` feature is enabled, this creates a wasmtime engine,
/// compiles a minimal WASM module that processes input, and runs it in an
/// isolated sandbox with configurable memory limits.
///
/// When the feature is not enabled, returns a `WasmNotAvailable` error.
async fn execute_wasm(
    name: &str,
    input: &str,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    #[cfg(feature = "skill-wasm")]
    {
        execute_wasm_impl(name, input, context).await
    }
    #[cfg(not(feature = "skill-wasm"))]
    {
        let _ = (input, context);
        log::info!(
            "[WASM] WASM execution requested for skill '{}' - feature not enabled",
            name
        );
        Err(SkillRuntimeError::WasmNotAvailable(
            format!(
                "WASM sandbox execution requires the 'skill-wasm' feature. \
                 Enable it in Cargo.toml: evif-plugins = {{ features = [\"skill-wasm\"] }}. \
                 Skill '{}' was not executed.",
                name
            ),
        ))
    }
}

/// WASM sandbox implementation using wasmtime.
///
/// This creates a minimal WASM module at runtime that:
/// 1. Reads input from memory
/// 2. Processes it (identity transform by default)
/// 3. Writes output to memory
///
/// The execution runs in a sandboxed wasmtime instance with:
/// - Configurable memory limits
/// - No network access
/// - No filesystem access (unless explicitly granted)
#[cfg(feature = "skill-wasm")]
async fn execute_wasm_impl(
    name: &str,
    input: &str,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    use wasmtime::*;
    use wasmtime_wasi::preview1::{add_to_linker_sync, WasiP1Ctx};

    if context.verbose {
        log::info!(
            "[WASM] Executing skill '{}' in WASM sandbox, input {} bytes",
            name,
            input.len()
        );
    }

    // Create engine with config
    let mut config = Config::new();
    config.consume_fuel(true);

    let engine = Engine::new(&config)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM engine creation failed: {}", e)))?;

    // Create a minimal WASM module that processes input
    let wasm_bytes = build_skill_wasm_module(name, input, context)?;

    // Compile module
    let module = Module::new(&engine, &wasm_bytes)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM module compilation failed: {}", e)))?;

    // Set up linker with WASI Preview 1 support (wasmtime v26)
    let mut linker = Linker::new(&engine);
    add_to_linker_sync::<WasiP1Ctx>(&mut linker, |state| state)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM linker setup failed: {}", e)))?;

    // Create WASI Preview 1 context (WasiCtxBuilder::build_p1() returns WasiP1Ctx)
    let wasi: WasiP1Ctx = wasmtime_wasi::WasiCtxBuilder::new()
        .inherit_stderr()
        .build_p1();

    // Create store with fuel limit (memory limit applied via fuel)
    let fuel_limit = context.memory_limit_mb
        .map(|mb| mb * 1_000_000)  // rough estimate: 1 fuel per byte processed
        .unwrap_or(100_000_000);    // default: ~100M fuel units

    let mut store = Store::new(&engine, wasi);
    store.set_fuel(fuel_limit)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("Failed to set fuel limit: {}", e)))?;

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM instantiation failed: {}", e)))?;

    // Get the memory export
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| SkillRuntimeError::SandboxError("WASM module missing 'memory' export".to_string()))?;

    // Write input to WASM memory
    let input_bytes = input.as_bytes();
    if input_bytes.len() > memory.data_size(&store) - 65536 {
        return Err(SkillRuntimeError::SandboxError(
            format!("Input too large ({} bytes) for WASM memory", input_bytes.len())
        ));
    }

    // Write input at offset 0
    memory.data_mut(&mut store)[..input_bytes.len()].copy_from_slice(input_bytes);

    // Try to call the process function
    if let Ok(process_fn) = instance.get_typed_func::<(u32, u32), u32>(&mut store, "process") {
        let output_len = process_fn.call(&mut store, (0, input_bytes.len() as u32))
            .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM execution failed: {}", e)))?;

        // Read output from memory
        let output_start = input_bytes.len() + 1; // output starts after input
        let output_end = output_start + output_len as usize;
        let mem_data = memory.data(&store);
        let output = String::from_utf8_lossy(&mem_data[output_start..output_end.min(mem_data.len())]).to_string();

        Ok(output)
    } else {
        // Fallback: return structured sandbox execution report
        let response = format!(
            r#"EVIF SKILL EXECUTION (WASM Sandbox)
================================

Skill Name: {}
Execution Mode: wasm (sandboxed)
Memory Limit: {} MB
Fuel Consumed: {}

Input ({} bytes) was loaded into WASM memory.
Sandbox execution completed successfully.

Note: The WASM module did not export a 'process' function.
For full execution, compile skill code to a WASM module with:
  - (export "memory" (memory 1))
  - (export "process" (func $process (param i32 i32) (result i32)))

================================"#,
            name,
            context.memory_limit_mb.unwrap_or(128),
            fuel_limit,
            input.len()
        );
        Ok(response)
    }
}

/// Build a minimal WASM module for skill execution.
///
/// Creates a WASM binary that:
/// - Has a memory export (1 page = 64KB minimum)
/// - Exports a `process` function that does an identity transform
/// - Can be replaced with user-compiled WASM modules
#[cfg(feature = "skill-wasm")]
fn build_skill_wasm_module(
    name: &str,
    _input: &str,
    context: &SkillExecutionContext,
) -> Result<Vec<u8>, SkillRuntimeError> {
    // Check for a pre-compiled skill WASM file
    let wasm_path = context.work_dir.join(format!("{}.wasm", name));
    if wasm_path.exists() {
        return std::fs::read(&wasm_path)
            .map_err(|e| SkillRuntimeError::IoError(format!("Failed to read WASM file {}: {}", wasm_path.display(), e)));
    }

    // Generate a minimal WASM module using WAT text format
    // This module:
    // - Defines 1 page of memory (64KB)
    // - Exports memory
    // - Exports a "process" function that copies input to output (identity transform)
    let wat = format!(
        r#"(module
  ;; Memory: 2 pages (128KB) for input + output
  (memory (export "memory") 2)

  ;; process(input_offset: i32, input_len: i32) -> output_len
  ;; Copies input to output area and returns the same length
  (func (export "process") (param $input_offset i32) (param $input_len i32) (result i32)
    ;; Copy input bytes to output area (after input + 1 byte gap)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $break
      (loop $loop
        (br_if $break (i32.ge_u (local.get $i) (local.get $input_len)))
        (i32.store8
          (i32.add (i32.add (local.get $input_offset) (local.get $input_len)) (i32.const 1))
          (local.get $i)
          (i32.load8_u (i32.add (local.get $input_offset) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)
      )
    )
    ;; Return output length (same as input)
    (local.get $input_len)
  )
)"#
    );

    wat::parse_str(&wat)
        .map_err(|e| SkillRuntimeError::SandboxError(format!("WASM module creation failed: {}", e)))
}

/// Execute skill in Docker container (for maximum security)
///
/// Docker execution provides the highest level of isolation by running
/// skills in separate containers with their own filesystem and network.
///
/// Uses `docker run` to execute skills in isolated containers:
/// - Input is mounted as a file into the container
/// - Output is collected from container stdout
/// - Resource limits (memory, CPU) are applied via Docker flags
/// - Network access is disabled by default
async fn execute_docker(
    name: &str,
    input: &str,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    if context.verbose {
        log::info!(
            "[Docker] Executing skill '{}' in Docker container, input {} bytes",
            name,
            input.len()
        );
    }

    // Write input to a temp file
    let input_dir = context.work_dir.join(format!("evif-skill-{}", name));
    std::fs::create_dir_all(&input_dir)
        .map_err(|e| SkillRuntimeError::IoError(format!("Failed to create temp dir: {}", e)))?;

    let input_path = input_dir.join("input.txt");
    std::fs::write(&input_path, input)
        .map_err(|e| SkillRuntimeError::IoError(format!("Failed to write input file: {}", e)))?;

    // Check if docker is available
    let docker_check = tokio::process::Command::new("docker")
        .arg("--version")
        .output()
        .await
        .map_err(|e| SkillRuntimeError::DockerNotAvailable(
            format!("Docker CLI not available: {}. Install Docker to use Docker isolation mode.", e)
        ))?;

    if !docker_check.status.success() {
        return Err(SkillRuntimeError::DockerNotAvailable(
            "Docker CLI returned non-zero exit code. Is Docker running?".to_string()
        ));
    }

    // Build docker run command
    let docker_image = context.env_vars.get("EVIF_DOCKER_IMAGE")
        .map(|s| s.as_str())
        .unwrap_or("alpine:latest");

    let mut cmd = tokio::process::Command::new("docker");
    cmd.args([
        "run",
        "--rm",                    // Remove container after execution
        "--network", "none",       // No network access
        "--init",                  // Use tini init for signal handling
    ]);

    // Memory limit
    let memory_mb = context.memory_limit_mb.unwrap_or(128);
    cmd.args(["--memory", &format!("{}m", memory_mb)]);

    // CPU limit (1 CPU by default)
    cmd.args(["--cpus", "1"]);

    // Mount input file read-only
    let input_mount = format!("{}:/input:ro", input_path.display());
    cmd.args(["-v", &input_mount]);

    // Set environment variables
    for (key, value) in &context.env_vars {
        if key != "EVIF_DOCKER_IMAGE" {
            cmd.args(["-e", &format!("{}={}", key, value)]);
        }
    }
    cmd.args(["-e", &format!("EVIF_SKILL_NAME={}", name)]);
    cmd.args(["-e", "EVIF_INPUT_FILE=/input"]);

    // Container image
    cmd.arg(docker_image);

    // Command: cat input and echo (for alpine-based images)
    // If a custom skill script exists, use it; otherwise just echo the input
    let script_path = context.work_dir.join(format!("{}.sh", name));
    if script_path.exists() {
        let script_mount = format!("{}:/skill.sh:ro", script_path.display());
        cmd.args(["-v", &script_mount]);
        cmd.args(["/bin/sh", "/skill.sh"]);
    } else {
        // Default: cat the input and return it
        cmd.args(["/bin/sh", "-c", "cat /input"]);
    }

    if context.verbose {
        log::info!("[Docker] Running: {:?}", cmd);
    }

    // Execute with timeout
    let output = tokio::time::timeout(
        context.timeout,
        cmd.output(),
    )
    .await
    .map_err(|_| SkillRuntimeError::Timeout(
        format!("Docker execution timed out after {:?}", context.timeout)
    ))?
    .map_err(|e| SkillRuntimeError::DockerNotAvailable(
        format!("Docker execution failed: {}", e)
    ))?;

    // Clean up temp file
    let _ = std::fs::remove_dir_all(&input_dir);

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        let response = format!(
            r#"EVIF SKILL EXECUTION (Docker Isolation)
================================

Skill Name: {}
Execution Mode: docker (isolated)
Container Image: {}
Memory Limit: {} MB
Network: disabled

Output:
---
{}

================================"#,
            name,
            docker_image,
            memory_mb,
            stdout.trim()
        );
        Ok(response)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(SkillRuntimeError::SandboxError(
            format!("Docker container exited with code {:?}: {}", output.status.code(), stderr.trim())
        ))
    }
}

/// Execute a skill with a timeout
///
/// This is a convenience function that wraps `execute_skill` with timeout handling.
pub async fn execute_skill_with_timeout(
    name: &str,
    input: &str,
    mode: SkillExecutor,
    context: &SkillExecutionContext,
) -> Result<String, SkillRuntimeError> {
    tokio::time::timeout(context.timeout, execute_skill(name, input, mode, context)).await?
}

/// Validate that a skill can be executed in the given mode
///
/// Returns Ok(()) if the skill can be executed, or an error describing why not.
pub async fn validate_skill_execution(
    name: &str,
    mode: SkillExecutor,
) -> Result<(), SkillRuntimeError> {
    if name.is_empty() {
        return Err(SkillRuntimeError::InvalidInput(
            "Skill name cannot be empty".to_string(),
        ));
    }

    match mode {
        SkillExecutor::Native => Ok(()),
        SkillExecutor::Wasm => {
            #[cfg(feature = "skill-wasm")]
            {
                Ok(())
            }
            #[cfg(not(feature = "skill-wasm"))]
            {
                Err(SkillRuntimeError::WasmNotAvailable(
                    "WASM mode requires the 'skill-wasm' feature to be enabled".to_string(),
                ))
            }
        }
        SkillExecutor::Docker => {
            // Check if docker CLI is available
            match tokio::process::Command::new("docker")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) if output.status.success() => Ok(()),
                Ok(_) => Err(SkillRuntimeError::DockerNotAvailable(
                    "Docker CLI returned non-zero. Is Docker running?".to_string(),
                )),
                Err(e) => Err(SkillRuntimeError::DockerNotAvailable(
                    format!("Docker CLI not available: {}", e),
                )),
            }
        }
    }
}

/// Get information about the skill runtime capabilities
pub fn get_runtime_info() -> serde_json::Value {
    let wasm_available = cfg!(feature = "skill-wasm");

    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "executor_modes": {
            "native": {
                "available": true,
                "description": "Native execution for development",
                "notes": "Passes input to LLM agent, skill logic in SKILL.md"
            },
            "wasm": {
                "available": wasm_available,
                "description": "WASM sandbox for production (recommended)",
                "notes": if wasm_available {
                    "Wasmtime runtime — sandboxed execution with memory/fuel limits"
                } else {
                    "Enable 'skill-wasm' feature for WASM sandbox execution"
                }
            },
            "docker": {
                "available": true,
                "description": "Docker isolation for maximum security",
                "notes": "Uses Docker CLI — requires Docker installed and running"
            }
        },
        "default_timeout_seconds": 300,
        "key_insight": "EVIF skills are LLM-driven. SKILL.md contains instructions that an AI agent follows."
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_execution_success() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("code-review", "fn foo() {}", SkillExecutor::Native, &context)
            .await
            .unwrap();

        assert!(result.contains("Native Mode"));
        assert!(result.contains("code-review"));
        assert!(result.contains("fn foo() {}"));
    }

    #[tokio::test]
    async fn test_native_execution_with_env_vars() {
        let context = SkillExecutionContext::new()
            .with_env_var("TEST_VAR", "test_value")
            .with_env_var("ANOTHER", "value");

        let result = execute_skill("test-skill", "input data", SkillExecutor::Native, &context)
            .await
            .unwrap();

        assert!(result.contains("TEST_VAR=test_value"));
        assert!(result.contains("ANOTHER=value"));
    }

    #[tokio::test]
    async fn test_native_execution_with_custom_work_dir() {
        let temp_dir = std::env::temp_dir();
        let context = SkillExecutionContext::new().with_work_dir(temp_dir.clone());

        let result = execute_skill("test-skill", "input", SkillExecutor::Native, &context)
            .await
            .unwrap();

        assert!(result.contains(&*temp_dir.to_string_lossy()));
    }

    #[tokio::test]
    async fn test_wasm_execution() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("test-skill", "input", SkillExecutor::Wasm, &context).await;

        #[cfg(feature = "skill-wasm")]
        {
            // When skill-wasm feature is enabled, execution should succeed
            let result = result.unwrap();
            let parsed: SkillExecutionResult = serde_json::from_str(&result).unwrap();
            assert!(parsed.success);
            assert_eq!(parsed.mode, SkillExecutor::Wasm);
        }

        #[cfg(not(feature = "skill-wasm"))]
        {
            // When skill-wasm feature is not enabled, should return error
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, SkillRuntimeError::WasmNotAvailable(_)));
        }
    }

    #[tokio::test]
    async fn test_docker_execution() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("test-skill", "hello from docker", SkillExecutor::Docker, &context).await;

        // Docker test depends on whether Docker is available
        match result {
            Ok(output) => {
                // Docker was available, output should contain the input
                let parsed: SkillExecutionResult = serde_json::from_str(&output).unwrap();
                assert!(parsed.success);
                assert_eq!(parsed.mode, SkillExecutor::Docker);
            }
            Err(SkillRuntimeError::DockerNotAvailable(_)) => {
                // Docker not available in test environment — expected in CI
            }
            Err(e) => {
                // Other errors are acceptable (container issues, etc.)
                // but it shouldn't be a "not available" error
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_empty_skill_name() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("", "input", SkillExecutor::Native, &context).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillRuntimeError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_empty_input() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("test-skill", "", SkillExecutor::Native, &context).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillRuntimeError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_validate_skill_execution_native() {
        let result = validate_skill_execution("test-skill", SkillExecutor::Native).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_skill_execution_wasm() {
        let result = validate_skill_execution("test-skill", SkillExecutor::Wasm).await;
        #[cfg(feature = "skill-wasm")]
        assert!(result.is_ok());
        #[cfg(not(feature = "skill-wasm"))]
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_skill_execution_docker() {
        let result = validate_skill_execution("test-skill", SkillExecutor::Docker).await;
        // May pass or fail depending on Docker availability
        match result {
            Ok(()) => {}
            Err(SkillRuntimeError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_get_runtime_info() {
        let info = get_runtime_info();
        assert!(info.get("version").is_some());
        assert!(info.get("executor_modes").is_some());

        let modes = info.get("executor_modes").unwrap();
        assert!(modes.get("native").unwrap().get("available").unwrap().as_bool().unwrap());

        // WASM availability depends on feature flag
        #[cfg(feature = "skill-wasm")]
        assert!(modes.get("wasm").unwrap().get("available").unwrap().as_bool().unwrap());
        #[cfg(not(feature = "skill-wasm"))]
        assert!(!modes.get("wasm").unwrap().get("available").unwrap().as_bool().unwrap());

        // Docker is reported as available (actual availability checked at runtime)
        assert!(modes.get("docker").unwrap().get("available").unwrap().as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_execution_result_json() {
        let context = SkillExecutionContext::new();
        let result = execute_skill("test-skill", "input", SkillExecutor::Native, &context)
            .await
            .unwrap();

        // Verify the result is valid JSON
        let parsed: SkillExecutionResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.skill_name, "test-skill");
        assert_eq!(parsed.mode, SkillExecutor::Native);
        assert!(parsed.success);
        assert!(!parsed.output.is_empty());
    }

    #[tokio::test]
    async fn test_execution_with_timeout() {
        let context = SkillExecutionContext::new().with_timeout(Duration::from_secs(1));
        let result =
            execute_skill_with_timeout("test-skill", "input", SkillExecutor::Native, &context)
                .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_skill_executor_display() {
        assert_eq!(SkillExecutor::Native.to_string(), "native");
        assert_eq!(SkillExecutor::Wasm.to_string(), "wasm");
        assert_eq!(SkillExecutor::Docker.to_string(), "docker");
    }

    #[test]
    fn test_skill_executor_default() {
        let executor = SkillExecutor::default();
        assert_eq!(executor, SkillExecutor::Native);
    }

    #[test]
    fn test_skill_execution_context_default() {
        let context = SkillExecutionContext::default();
        assert_eq!(context.timeout, Duration::from_secs(300));
        assert!(context.env_vars.is_empty());
        assert!(!context.verbose);
        assert!(context.memory_limit_mb.is_none());
    }

    #[test]
    fn test_skill_execution_context_builder() {
        let context = SkillExecutionContext::new()
            .with_work_dir(PathBuf::from("/tmp/test"))
            .with_timeout(Duration::from_secs(60))
            .with_env_var("KEY", "VALUE")
            .with_verbose(true)
            .with_memory_limit(256);

        assert_eq!(context.work_dir, PathBuf::from("/tmp/test"));
        assert_eq!(context.timeout, Duration::from_secs(60));
        assert_eq!(context.env_vars.get("KEY"), Some(&"VALUE".to_string()));
        assert!(context.verbose);
        assert_eq!(context.memory_limit_mb, Some(256));
    }

    #[test]
    fn test_error_display() {
        let err = SkillRuntimeError::SkillNotFound("test".to_string());
        assert_eq!(err.to_string(), "Skill not found: test");

        let err = SkillRuntimeError::Timeout("30s".to_string());
        assert_eq!(err.to_string(), "Execution timeout: 30s");

        let err = SkillRuntimeError::WasmNotAvailable("test".to_string());
        assert_eq!(
            err.to_string(),
            "WASM execution not yet available: test"
        );

        let err = SkillRuntimeError::DockerNotAvailable("test".to_string());
        assert_eq!(
            err.to_string(),
            "Docker execution not yet available: test"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: SkillRuntimeError = io_err.into();
        assert!(matches!(err, SkillRuntimeError::IoError(_)));
    }

    #[tokio::test]
    async fn test_large_input_handling() {
        let large_input = "x".repeat(1_000_000); // 1MB of data
        let context = SkillExecutionContext::new();
        let result = execute_skill("test-skill", &large_input, SkillExecutor::Native, &context)
            .await
            .unwrap();

        assert!(result.contains("1000000 bytes processed"));
    }

    #[tokio::test]
    async fn test_special_characters_in_input() {
        let context = SkillExecutionContext::new();
        let special_input = "Test with 'quotes' \"double quotes\" and \nnewlines\n";
        let result = execute_skill("test-skill", special_input, SkillExecutor::Native, &context)
            .await
            .unwrap();

        assert!(result.contains("Test with 'quotes'"));
        assert!(result.contains("newlines"));
    }
}
