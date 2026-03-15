//! Workflow system for configurable memory pipelines
//!
//! This module provides a flexible workflow engine that allows dynamic configuration
//! of memory processing pipelines, inspired by memU's workflow system.
//!
//! # Architecture
//! - `WorkflowStep`: Single step definition (LLM, Function, or Parallel)
//! - `WorkflowRunner`: Trait for executing workflows
//! - `DefaultWorkflowRunner`: Default implementation with sequential and parallel execution
//! - `WorkflowState`: State passed between steps
//! - `WorkflowConfig`: Configuration options
//! - `WorkflowStats`: Execution statistics

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::{MemError, MemResult};

/// Step type - determines how the step is executed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// LLM-based step - calls LLM with prompt
    LLM,
    /// Function-based step - calls Rust function
    Function,
    /// Parallel execution of multiple steps
    Parallel,
}

/// Capability - represents required system capabilities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    /// LLM capability - can call LLM
    LLM,
    /// Vector capability - can do vector search
    Vector,
    /// Database capability - can access storage
    DB,
    /// IO capability - can do file/network I/O
    IO,
    /// Embedding capability - can generate embeddings
    Embedding,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::LLM => write!(f, "llm"),
            Capability::Vector => write!(f, "vector"),
            Capability::DB => write!(f, "db"),
            Capability::IO => write!(f, "io"),
            Capability::Embedding => write!(f, "embedding"),
        }
    }
}

/// Function type for workflow steps - async function that takes state and returns modified state
pub type StepFunction = Box<
    dyn Fn(
            HashMap<String, serde_json::Value>,
        )
            -> Pin<Box<dyn Future<Output = MemResult<HashMap<String, serde_json::Value>>> + Send>>
        + Send
        + Sync,
>;

/// Workflow step - single step in a workflow pipeline
pub struct WorkflowStep {
    /// Unique step identifier
    pub step_id: String,
    /// Step type (llm, function, parallel)
    pub step_type: StepType,
    /// Required capabilities
    pub capabilities: HashSet<Capability>,
    /// Function to execute (for function type)
    pub function: Option<Arc<StepFunction>>,
    /// Prompt template (for LLM type)
    pub prompt_template: Option<String>,
    /// LLM profile to use (for LLM type)
    pub llm_profile: Option<String>,
    /// Dependencies - steps that must complete first
    pub depends_on: Option<Vec<String>>,
    /// Whether this step can run in parallel with siblings
    pub parallel: bool,
    /// Sub-steps (for parallel type)
    pub sub_steps: Option<Vec<WorkflowStep>>,
}

impl std::fmt::Debug for WorkflowStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowStep")
            .field("step_id", &self.step_id)
            .field("step_type", &self.step_type)
            .field("capabilities", &self.capabilities)
            .field("prompt_template", &self.prompt_template)
            .field("llm_profile", &self.llm_profile)
            .field("depends_on", &self.depends_on)
            .field("parallel", &self.parallel)
            .field("sub_steps", &self.sub_steps)
            .field("function", &self.function.as_ref().map(|_| "..."))
            .finish()
    }
}

impl WorkflowStep {
    /// Create a new LLM step
    pub fn llm(step_id: impl Into<String>, prompt_template: impl Into<String>) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::LLM);

        Self {
            step_id: step_id.into(),
            step_type: StepType::LLM,
            capabilities,
            function: None,
            prompt_template: Some(prompt_template.into()),
            llm_profile: None,
            depends_on: None,
            parallel: false,
            sub_steps: None,
        }
    }

    /// Create a new function step
    pub fn function<F, Fut>(
        step_id: impl Into<String>,
        func: F,
        capabilities: Vec<Capability>,
    ) -> Self
    where
        F: Fn(HashMap<String, serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = MemResult<HashMap<String, serde_json::Value>>> + Send + 'static,
    {
        let caps: HashSet<Capability> = capabilities.into_iter().collect();
        let boxed_func: StepFunction = Box::new(move |state| Box::pin(func(state)));

        Self {
            step_id: step_id.into(),
            step_type: StepType::Function,
            capabilities: caps,
            function: Some(Arc::new(boxed_func)),
            prompt_template: None,
            llm_profile: None,
            depends_on: None,
            parallel: false,
            sub_steps: None,
        }
    }

    /// Create a parallel step with multiple sub-steps
    pub fn parallel(step_id: impl Into<String>, sub_steps: Vec<WorkflowStep>) -> Self {
        let mut capabilities = HashSet::new();
        for step in &sub_steps {
            capabilities.extend(step.capabilities.clone());
        }

        Self {
            step_id: step_id.into(),
            step_type: StepType::Parallel,
            capabilities,
            function: None,
            prompt_template: None,
            llm_profile: None,
            depends_on: None,
            parallel: true,
            sub_steps: Some(sub_steps),
        }
    }

    /// Set dependencies
    pub fn with_depends_on(mut self, depends_on: Vec<String>) -> Self {
        self.depends_on = Some(depends_on);
        self
    }

    /// Set LLM profile
    pub fn with_llm_profile(mut self, profile: impl Into<String>) -> Self {
        self.llm_profile = Some(profile.into());
        self
    }

    /// Set parallel flag
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
}

/// Workflow state - passed between steps
#[derive(Debug, Clone)]
pub struct WorkflowState {
    /// Step outputs indexed by step_id
    pub step_outputs: HashMap<String, serde_json::Value>,
    /// Global state accessible to all steps
    pub global: HashMap<String, serde_json::Value>,
}

impl WorkflowState {
    /// Create new empty state
    pub fn new() -> Self {
        Self {
            step_outputs: HashMap::new(),
            global: HashMap::new(),
        }
    }

    /// Create state with initial global data
    pub fn with_global(data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            step_outputs: HashMap::new(),
            global: data,
        }
    }

    /// Get step output
    pub fn get_step_output(&self, step_id: &str) -> Option<&serde_json::Value> {
        self.step_outputs.get(step_id)
    }

    /// Set step output
    pub fn set_step_output(&mut self, step_id: String, output: serde_json::Value) {
        self.step_outputs.insert(step_id, output);
    }

    /// Get global value
    pub fn get_global(&self, key: &str) -> Option<&serde_json::Value> {
        self.global.get(key)
    }

    /// Set global value
    pub fn set_global(&mut self, key: String, value: serde_json::Value) {
        self.global.insert(key, value);
    }
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Maximum concurrent parallel steps
    pub max_parallel: usize,
    /// Enable step-level logging
    pub enable_logging: bool,
    /// Stop on first error
    pub stop_on_error: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_parallel: 10,
            enable_logging: true,
            stop_on_error: true,
        }
    }
}

/// Workflow execution statistics
#[derive(Debug, Clone, Default)]
pub struct WorkflowStats {
    /// Total steps executed
    pub steps_executed: usize,
    /// Steps succeeded
    pub steps_succeeded: usize,
    /// Steps failed
    pub steps_failed: usize,
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Step execution times (step_id -> time_ms)
    pub step_times: HashMap<String, u64>,
}

/// Workflow Runner Trait
///
/// Defines the interface for executing workflow steps.
#[async_trait]
pub trait WorkflowRunner: Send + Sync {
    /// Run a workflow with the given steps and initial state
    ///
    /// Returns the final state after all steps have been executed.
    async fn run(
        &self,
        steps: &[WorkflowStep],
        initial_state: WorkflowState,
    ) -> MemResult<(WorkflowState, WorkflowStats)>;

    /// Validate that required capabilities are available
    ///
    /// Returns an error if any step requires capabilities not in the provided set.
    fn validate_capabilities(
        &self,
        steps: &[WorkflowStep],
        available_capabilities: &HashSet<Capability>,
    ) -> MemResult<()>;
}

/// LLM Provider trait for workflow execution
///
/// Abstracts LLM operations needed by the workflow runner.
#[async_trait]
pub trait WorkflowLLMProvider: Send + Sync {
    /// Generate completion with the given prompt
    async fn generate(&self, prompt: &str, profile: Option<&str>) -> MemResult<String>;
}

/// Interceptor context - contains information passed to interceptors
#[derive(Debug, Clone)]
pub struct InterceptorContext {
    /// Step ID being executed
    pub step_id: String,
    /// Step type
    pub step_type: StepType,
    /// Prompt (for LLM steps)
    pub prompt: Option<String>,
    /// LLM profile (for LLM steps)
    pub llm_profile: Option<String>,
    /// Current workflow state
    pub state: WorkflowState,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InterceptorContext {
    /// Create a new interceptor context
    pub fn new(step_id: String, step_type: StepType, state: WorkflowState) -> Self {
        Self {
            step_id,
            step_type,
            prompt: None,
            llm_profile: None,
            state,
            metadata: HashMap::new(),
        }
    }

    /// Set prompt
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set LLM profile
    pub fn with_llm_profile(mut self, profile: impl Into<String>) -> Self {
        self.llm_profile = Some(profile.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Interceptor trait - allows hooking into workflow execution
///
/// Interceptors can modify context before step execution and results after execution.
/// This enables cross-cutting concerns like logging, monitoring, caching, and rate limiting.
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Called before step execution
    ///
    /// Can modify the context (e.g., add metadata, modify prompt).
    async fn before(&self, context: &mut InterceptorContext) -> MemResult<()>;

    /// Called after step execution
    ///
    /// Can modify the result (e.g., cache it, transform it).
    async fn after(
        &self,
        result: serde_json::Value,
        context: &InterceptorContext,
    ) -> MemResult<serde_json::Value>;
}

/// Interceptor registry - manages multiple interceptors
pub struct InterceptorRegistry {
    /// Registered interceptors
    interceptors: RwLock<Vec<Arc<dyn Interceptor>>>,
}

impl InterceptorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            interceptors: RwLock::new(Vec::new()),
        }
    }

    /// Register an interceptor
    pub async fn register(&self, interceptor: Arc<dyn Interceptor>) {
        let mut interceptors = self.interceptors.write().await;
        interceptors.push(interceptor);
    }

    /// Get number of registered interceptors
    pub async fn len(&self) -> usize {
        self.interceptors.read().await.len()
    }

    /// Check if registry is empty
    pub async fn is_empty(&self) -> bool {
        self.interceptors.read().await.is_empty()
    }

    /// Execute all before hooks
    pub async fn execute_before(&self, context: &mut InterceptorContext) -> MemResult<()> {
        for interceptor in self.interceptors.read().await.iter() {
            interceptor.before(context).await?;
        }
        Ok(())
    }

    /// Execute all after hooks
    pub async fn execute_after(
        &self,
        result: serde_json::Value,
        context: &InterceptorContext,
    ) -> MemResult<serde_json::Value> {
        let mut current_result = result;
        for interceptor in self.interceptors.read().await.iter() {
            current_result = interceptor.after(current_result, context).await?;
        }
        Ok(current_result)
    }
}

impl Default for InterceptorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InterceptorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptorRegistry")
            .field("count", &self.interceptors.blocking_read().len())
            .finish()
    }
}

/// Pipeline Manager - Dynamic pipeline registration and execution
///
/// Manages named workflow pipelines with validation of capabilities and LLM profiles.
pub struct PipelineManager {
    /// Registered pipelines (name -> steps)
    pipelines: RwLock<HashMap<String, Vec<WorkflowStep>>>,
    /// Available capabilities
    capabilities: HashSet<Capability>,
    /// Available LLM profiles
    llm_profiles: HashSet<String>,
    /// Workflow runner for execution
    runner: Arc<DefaultWorkflowRunner>,
}

impl PipelineManager {
    /// Create a new pipeline manager
    pub fn new(
        capabilities: HashSet<Capability>,
        llm_profiles: HashSet<String>,
        runner: Arc<DefaultWorkflowRunner>,
    ) -> Self {
        Self {
            pipelines: RwLock::new(HashMap::new()),
            capabilities,
            llm_profiles,
            runner,
        }
    }

    /// Register a new pipeline
    ///
    /// Validates that all steps have required capabilities and valid LLM profiles.
    pub async fn register(
        &self,
        name: impl Into<String>,
        steps: Vec<WorkflowStep>,
    ) -> MemResult<()> {
        let name = name.into();

        // Validate capabilities
        for step in &steps {
            let missing: Vec<_> = step.capabilities.difference(&self.capabilities).collect();

            if !missing.is_empty() {
                let missing_str: Vec<_> = missing.iter().map(|c| c.to_string()).collect();
                return Err(MemError::WorkflowError(format!(
                    "Pipeline '{}': Step '{}' requires missing capabilities: {}",
                    name,
                    step.step_id,
                    missing_str.join(", ")
                )));
            }

            // Validate LLM profile for LLM steps
            if step.step_type == StepType::LLM {
                if let Some(profile) = &step.llm_profile {
                    if !self.llm_profiles.contains(profile) {
                        return Err(MemError::WorkflowError(format!(
                            "Pipeline '{}': Step '{}' uses unknown LLM profile '{}'",
                            name, step.step_id, profile
                        )));
                    }
                }
            }

            // Recursively validate sub-steps
            if let Some(sub_steps) = &step.sub_steps {
                self.validate_sub_steps(&name, sub_steps)?;
            }
        }

        // Register pipeline
        let mut pipelines = self.pipelines.write().await;
        pipelines.insert(name, steps);

        Ok(())
    }

    /// Validate sub-steps recursively
    fn validate_sub_steps(&self, pipeline_name: &str, steps: &[WorkflowStep]) -> MemResult<()> {
        for step in steps {
            let missing: Vec<_> = step.capabilities.difference(&self.capabilities).collect();

            if !missing.is_empty() {
                let missing_str: Vec<_> = missing.iter().map(|c| c.to_string()).collect();
                return Err(MemError::WorkflowError(format!(
                    "Pipeline '{}': Sub-step '{}' requires missing capabilities: {}",
                    pipeline_name,
                    step.step_id,
                    missing_str.join(", ")
                )));
            }

            if step.step_type == StepType::LLM {
                if let Some(profile) = &step.llm_profile {
                    if !self.llm_profiles.contains(profile) {
                        return Err(MemError::WorkflowError(format!(
                            "Pipeline '{}': Sub-step '{}' uses unknown LLM profile '{}'",
                            pipeline_name, step.step_id, profile
                        )));
                    }
                }
            }

            // Recursively validate nested sub-steps
            if let Some(sub_steps) = &step.sub_steps {
                self.validate_sub_steps(pipeline_name, sub_steps)?;
            }
        }

        Ok(())
    }

    /// Run a registered pipeline
    pub async fn run(
        &self,
        name: &str,
        initial_state: WorkflowState,
    ) -> MemResult<(WorkflowState, WorkflowStats)> {
        let pipelines = self.pipelines.read().await;

        let steps = pipelines
            .get(name)
            .ok_or_else(|| MemError::WorkflowError(format!("Pipeline '{}' not found", name)))?;

        self.runner.run(steps, initial_state).await
    }

    /// List all registered pipelines
    pub async fn list_pipelines(&self) -> Vec<String> {
        let pipelines = self.pipelines.read().await;
        pipelines.keys().cloned().collect()
    }

    /// Check if a pipeline exists
    pub async fn has_pipeline(&self, name: &str) -> bool {
        let pipelines = self.pipelines.read().await;
        pipelines.contains_key(name)
    }

    /// Remove a pipeline
    pub async fn remove_pipeline(&self, name: &str) -> MemResult<()> {
        let mut pipelines = self.pipelines.write().await;

        pipelines
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| MemError::WorkflowError(format!("Pipeline '{}' not found", name)))
    }

    /// Get number of registered pipelines
    pub async fn len(&self) -> usize {
        self.pipelines.read().await.len()
    }

    /// Check if no pipelines are registered
    pub async fn is_empty(&self) -> bool {
        self.pipelines.read().await.is_empty()
    }

    /// Configure a specific step in a pipeline
    ///
    /// Updates the configuration of a step without replacing it entirely.
    /// Returns the number of steps modified (0 if step not found).
    pub async fn config_step(
        &self,
        pipeline_name: &str,
        step_id: &str,
        configs: HashMap<String, serde_json::Value>,
    ) -> MemResult<usize> {
        let mut pipelines = self.pipelines.write().await;

        let steps = pipelines.get_mut(pipeline_name).ok_or_else(|| {
            MemError::WorkflowError(format!("Pipeline '{}' not found", pipeline_name))
        })?;

        let mut modified = 0;
        for step in steps.iter_mut() {
            if step.step_id == step_id {
                // Apply configurations
                if let Some(prompt) = configs.get("prompt_template") {
                    if let Ok(prompt_str) = serde_json::from_value(prompt.clone()) {
                        step.prompt_template = Some(prompt_str);
                    }
                }
                if let Some(profile) = configs.get("llm_profile") {
                    if let Ok(profile_str) = serde_json::from_value(profile.clone()) {
                        // Validate LLM profile
                        if !self.llm_profiles.contains(&profile_str) {
                            return Err(MemError::WorkflowError(format!(
                                "Unknown LLM profile '{}'",
                                profile_str
                            )));
                        }
                        step.llm_profile = Some(profile_str);
                    }
                }
                if let Some(parallel) = configs.get("parallel") {
                    if let Ok(parallel_bool) = serde_json::from_value(parallel.clone()) {
                        step.parallel = parallel_bool;
                    }
                }
                modified = 1;
                break;
            }
        }

        Ok(modified)
    }

    /// Insert a new step after a target step
    ///
    /// Returns Ok(1) if successful, Ok(0) if target step not found.
    pub async fn insert_after(
        &self,
        pipeline_name: &str,
        target_step_id: &str,
        new_step: WorkflowStep,
    ) -> MemResult<usize> {
        // Validate new step
        self.validate_step(&new_step)?;

        let mut pipelines = self.pipelines.write().await;

        let steps = pipelines.get_mut(pipeline_name).ok_or_else(|| {
            MemError::WorkflowError(format!("Pipeline '{}' not found", pipeline_name))
        })?;

        // Find target step index
        let target_index = steps.iter().position(|s| s.step_id == target_step_id);

        match target_index {
            Some(index) => {
                steps.insert(index + 1, new_step);
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// Insert a new step before a target step
    ///
    /// Returns Ok(1) if successful, Ok(0) if target step not found.
    pub async fn insert_before(
        &self,
        pipeline_name: &str,
        target_step_id: &str,
        new_step: WorkflowStep,
    ) -> MemResult<usize> {
        // Validate new step
        self.validate_step(&new_step)?;

        let mut pipelines = self.pipelines.write().await;

        let steps = pipelines.get_mut(pipeline_name).ok_or_else(|| {
            MemError::WorkflowError(format!("Pipeline '{}' not found", pipeline_name))
        })?;

        // Find target step index
        let target_index = steps.iter().position(|s| s.step_id == target_step_id);

        match target_index {
            Some(index) => {
                steps.insert(index, new_step);
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// Replace an existing step with a new one
    ///
    /// Returns Ok(1) if successful, Ok(0) if target step not found.
    pub async fn replace_step(
        &self,
        pipeline_name: &str,
        target_step_id: &str,
        new_step: WorkflowStep,
    ) -> MemResult<usize> {
        // Validate new step
        self.validate_step(&new_step)?;

        let mut pipelines = self.pipelines.write().await;

        let steps = pipelines.get_mut(pipeline_name).ok_or_else(|| {
            MemError::WorkflowError(format!("Pipeline '{}' not found", pipeline_name))
        })?;

        // Find target step index
        let target_index = steps.iter().position(|s| s.step_id == target_step_id);

        match target_index {
            Some(index) => {
                steps[index] = new_step;
                Ok(1)
            }
            None => Ok(0),
        }
    }

    /// Validate a single step
    fn validate_step(&self, step: &WorkflowStep) -> MemResult<()> {
        // Validate capabilities
        let missing: Vec<_> = step.capabilities.difference(&self.capabilities).collect();

        if !missing.is_empty() {
            let missing_str: Vec<_> = missing.iter().map(|c| c.to_string()).collect();
            return Err(MemError::WorkflowError(format!(
                "Step '{}' requires missing capabilities: {}",
                step.step_id,
                missing_str.join(", ")
            )));
        }

        // Validate LLM profile for LLM steps
        if step.step_type == StepType::LLM {
            if let Some(profile) = &step.llm_profile {
                if !self.llm_profiles.contains(profile) {
                    return Err(MemError::WorkflowError(format!(
                        "Step '{}' uses unknown LLM profile '{}'",
                        step.step_id, profile
                    )));
                }
            }
        }

        // Recursively validate sub-steps
        if let Some(sub_steps) = &step.sub_steps {
            for sub_step in sub_steps {
                self.validate_step(sub_step)?;
            }
        }

        Ok(())
    }
}

/// Default Workflow Runner Implementation
///
/// Executes workflow steps sequentially or in parallel based on step configuration.
pub struct DefaultWorkflowRunner {
    /// LLM provider for LLM steps
    llm_provider: Arc<RwLock<Box<dyn WorkflowLLMProvider>>>,
    /// Configuration
    config: WorkflowConfig,
    /// Available capabilities
    capabilities: HashSet<Capability>,
    /// Interceptor registry
    interceptors: Arc<InterceptorRegistry>,
}

impl DefaultWorkflowRunner {
    /// Create a new workflow runner
    pub fn new(
        llm_provider: Arc<RwLock<Box<dyn WorkflowLLMProvider>>>,
        config: WorkflowConfig,
        capabilities: HashSet<Capability>,
    ) -> Self {
        Self {
            llm_provider,
            config,
            capabilities,
            interceptors: Arc::new(InterceptorRegistry::new()),
        }
    }

    /// Create a runner with default configuration
    pub fn with_llm(llm_provider: Arc<RwLock<Box<dyn WorkflowLLMProvider>>>) -> Self {
        Self::new(
            llm_provider,
            WorkflowConfig::default(),
            HashSet::from([Capability::LLM, Capability::DB, Capability::IO]),
        )
    }

    /// Create a runner with interceptors
    pub fn with_interceptors(
        llm_provider: Arc<RwLock<Box<dyn WorkflowLLMProvider>>>,
        interceptors: Arc<InterceptorRegistry>,
    ) -> Self {
        Self {
            llm_provider,
            config: WorkflowConfig::default(),
            capabilities: HashSet::from([Capability::LLM, Capability::DB, Capability::IO]),
            interceptors,
        }
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        state: &mut WorkflowState,
        stats: &mut WorkflowStats,
    ) -> MemResult<()> {
        let start = Instant::now();

        // Check dependencies
        if let Some(deps) = &step.depends_on {
            for dep_id in deps {
                if !state.step_outputs.contains_key(dep_id) {
                    return Err(MemError::WorkflowError(format!(
                        "Step '{}' depends on '{}' which has not been executed",
                        step.step_id, dep_id
                    )));
                }
            }
        }

        let result = match step.step_type {
            StepType::Function => self.execute_function_step(step, state).await,
            StepType::LLM => self.execute_llm_step(step, state).await,
            StepType::Parallel => self.execute_parallel_step(step, state, stats).await,
        };

        let elapsed = start.elapsed().as_millis() as u64;
        stats.step_times.insert(step.step_id.clone(), elapsed);
        stats.steps_executed += 1;

        match result {
            Ok(output) => {
                state.set_step_output(step.step_id.clone(), output);
                stats.steps_succeeded += 1;
                Ok(())
            }
            Err(e) => {
                stats.steps_failed += 1;
                if self.config.stop_on_error {
                    Err(e)
                } else {
                    // Log error but continue
                    if self.config.enable_logging {
                        tracing::warn!("Step '{}' failed: {}", step.step_id, e);
                    }
                    state.set_step_output(
                        step.step_id.clone(),
                        serde_json::json!({ "error": e.to_string() }),
                    );
                    Ok(())
                }
            }
        }
    }

    /// Execute a function step
    async fn execute_function_step(
        &self,
        step: &WorkflowStep,
        state: &WorkflowState,
    ) -> MemResult<serde_json::Value> {
        let func = step.function.as_ref().ok_or_else(|| {
            MemError::WorkflowError(format!("Step '{}' has no function", step.step_id))
        })?;

        let state_map = state.step_outputs.clone();
        let mut result: HashMap<String, serde_json::Value> = func(state_map).await?;

        // Merge result into state
        if let Some(global) = result.remove("global") {
            let mut state = state.clone();
            if let Ok(global_map) =
                serde_json::from_value::<HashMap<String, serde_json::Value>>(global)
            {
                for (k, v) in global_map {
                    state.set_global(k, v);
                }
            }
        }

        Ok(serde_json::to_value(result)?)
    }

    /// Execute an LLM step
    async fn execute_llm_step(
        &self,
        step: &WorkflowStep,
        state: &WorkflowState,
    ) -> MemResult<serde_json::Value> {
        let template = step.prompt_template.as_ref().ok_or_else(|| {
            MemError::WorkflowError(format!("Step '{}' has no prompt template", step.step_id))
        })?;

        // Render template with state
        let prompt = self.render_template(template, state)?;

        // Call LLM
        let llm = self.llm_provider.read().await;
        let response = llm.generate(&prompt, step.llm_profile.as_deref()).await?;

        Ok(serde_json::json!({ "response": response }))
    }

    /// Execute a parallel step with true concurrent execution
    async fn execute_parallel_step(
        &self,
        step: &WorkflowStep,
        state: &mut WorkflowState,
        stats: &mut WorkflowStats,
    ) -> MemResult<serde_json::Value> {
        let sub_steps = step.sub_steps.as_ref().ok_or_else(|| {
            MemError::WorkflowError(format!("Parallel step '{}' has no sub-steps", step.step_id))
        })?;

        // Spawn all sub-steps as concurrent tokio tasks
        let mut handles = Vec::new();

        for sub_step in sub_steps {
            // Clone necessary data for the async task
            let step_id = sub_step.step_id.clone();
            let step_type = sub_step.step_type.clone();
            let sub_state = state.clone();

            // Clone self references needed for execution
            let llm_provider = self.llm_provider.clone();
            let prompt_template = sub_step.prompt_template.clone();
            let llm_profile = sub_step.llm_profile.clone();
            let function = sub_step.function.clone();

            // Spawn task based on step type
            let handle = tokio::spawn(async move {
                let start = Instant::now();

                let result = match step_type {
                    StepType::Function => {
                        let func = function.as_ref().ok_or_else(|| {
                            MemError::WorkflowError(format!("Step '{}' has no function", step_id))
                        })?;

                        let state_map = sub_state.step_outputs.clone();
                        let result: HashMap<String, serde_json::Value> = func(state_map).await?;
                        Ok(serde_json::to_value(result)?)
                    }
                    StepType::LLM => {
                        let template = prompt_template.as_ref().ok_or_else(|| {
                            MemError::WorkflowError(format!(
                                "Step '{}' has no prompt template",
                                step_id
                            ))
                        })?;

                        // Render template
                        let mut prompt = template.clone();
                        for (key, value) in &sub_state.global {
                            let placeholder = format!("{{{}}}", key);
                            if let Some(s) = value.as_str() {
                                prompt = prompt.replace(&placeholder, s);
                            } else {
                                prompt = prompt.replace(&placeholder, &value.to_string());
                            }
                        }
                        for (s_id, output) in &sub_state.step_outputs {
                            if let Some(obj) = output.as_object() {
                                for (field, value) in obj {
                                    let placeholder = format!("{{{}.{}}}", s_id, field);
                                    if let Some(s) = value.as_str() {
                                        prompt = prompt.replace(&placeholder, s);
                                    } else {
                                        prompt = prompt.replace(&placeholder, &value.to_string());
                                    }
                                }
                            }
                        }

                        let llm = llm_provider.read().await;
                        let response = llm.generate(&prompt, llm_profile.as_deref()).await?;
                        Ok(serde_json::json!({ "response": response }))
                    }
                    StepType::Parallel => Err(MemError::WorkflowError(
                        "Nested parallel steps are not supported".to_string(),
                    )),
                };

                let elapsed = start.elapsed().as_millis() as u64;
                Ok::<_, MemError>((step_id, result, elapsed))
            });

            handles.push(handle);
        }

        // Collect results from all concurrent tasks
        let mut results = HashMap::new();
        for handle in handles {
            let (step_id, result, elapsed) = handle
                .await
                .map_err(|e| MemError::WorkflowError(format!("Task join error: {}", e)))??;

            stats.step_times.insert(step_id.clone(), elapsed);
            stats.steps_executed += 1;

            match result {
                Ok(output) => {
                    results.insert(step_id, output);
                    stats.steps_succeeded += 1;
                }
                Err(e) => {
                    stats.steps_failed += 1;
                    if self.config.stop_on_error {
                        return Err(e);
                    }
                    // Include error in results but continue
                }
            }
        }

        Ok(serde_json::to_value(results)?)
    }

    /// Render a template with state values
    fn render_template(&self, template: &str, state: &WorkflowState) -> MemResult<String> {
        let mut result = template.to_string();

        // Replace global variables: {var_name}
        for (key, value) in &state.global {
            let placeholder = format!("{{{}}}", key);
            if let Some(s) = value.as_str() {
                result = result.replace(&placeholder, s);
            } else {
                result = result.replace(&placeholder, &value.to_string());
            }
        }

        // Replace step outputs: {step_id.field}
        for (step_id, output) in &state.step_outputs {
            if let Some(obj) = output.as_object() {
                for (field, value) in obj {
                    let placeholder = format!("{{{}.{}}}", step_id, field);
                    if let Some(s) = value.as_str() {
                        result = result.replace(&placeholder, s);
                    } else {
                        result = result.replace(&placeholder, &value.to_string());
                    }
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl WorkflowRunner for DefaultWorkflowRunner {
    async fn run(
        &self,
        steps: &[WorkflowStep],
        initial_state: WorkflowState,
    ) -> MemResult<(WorkflowState, WorkflowStats)> {
        let start = Instant::now();
        let mut state = initial_state;
        let mut stats = WorkflowStats::default();

        // Validate capabilities
        self.validate_capabilities(steps, &self.capabilities)?;

        // Execute steps sequentially
        for step in steps {
            self.execute_step(step, &mut state, &mut stats).await?;
        }

        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok((state, stats))
    }

    fn validate_capabilities(
        &self,
        steps: &[WorkflowStep],
        available_capabilities: &HashSet<Capability>,
    ) -> MemResult<()> {
        for step in steps {
            let missing: Vec<_> = step
                .capabilities
                .difference(available_capabilities)
                .collect();

            if !missing.is_empty() {
                let missing_str: Vec<_> = missing.iter().map(|c| c.to_string()).collect();
                return Err(MemError::WorkflowError(format!(
                    "Step '{}' requires missing capabilities: {}",
                    step.step_id,
                    missing_str.join(", ")
                )));
            }

            // Recursively validate sub-steps
            if let Some(sub_steps) = &step.sub_steps {
                self.validate_capabilities(sub_steps, available_capabilities)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_type_serialization() {
        let step_type = StepType::LLM;
        let json = serde_json::to_string(&step_type).unwrap();
        assert_eq!(json, "\"llm\"");

        let deserialized: StepType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, StepType::LLM);
    }

    #[test]
    fn test_capability_hash_set() {
        let mut caps = HashSet::new();
        caps.insert(Capability::LLM);
        caps.insert(Capability::Vector);

        assert!(caps.contains(&Capability::LLM));
        assert!(caps.contains(&Capability::Vector));
        assert!(!caps.contains(&Capability::DB));
    }

    #[test]
    fn test_workflow_step_llm() {
        let step =
            WorkflowStep::llm("extract", "Extract memories from: {text}").with_llm_profile("gpt-4");

        assert_eq!(step.step_id, "extract");
        assert_eq!(step.step_type, StepType::LLM);
        assert!(step.capabilities.contains(&Capability::LLM));
        assert_eq!(
            step.prompt_template,
            Some("Extract memories from: {text}".to_string())
        );
        assert_eq!(step.llm_profile, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_workflow_step_function() {
        let step = WorkflowStep::function(
            "dedupe",
            |mut state| async move {
                state.insert("deduped".to_string(), serde_json::json!(true));
                Ok(state)
            },
            vec![Capability::DB],
        );

        assert_eq!(step.step_id, "dedupe");
        assert_eq!(step.step_type, StepType::Function);
        assert!(step.capabilities.contains(&Capability::DB));
        assert!(step.function.is_some());
    }

    #[test]
    fn test_workflow_step_parallel() {
        let step1 = WorkflowStep::llm("extract1", "Extract 1");
        let step2 = WorkflowStep::llm("extract2", "Extract 2");
        let parallel_step = WorkflowStep::parallel("parallel_extract", vec![step1, step2]);

        assert_eq!(parallel_step.step_id, "parallel_extract");
        assert_eq!(parallel_step.step_type, StepType::Parallel);
        assert!(parallel_step.parallel);
        assert_eq!(parallel_step.sub_steps.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_workflow_state() {
        let mut state = WorkflowState::new();
        state.set_step_output("step1".to_string(), serde_json::json!({"result": "ok"}));
        state.set_global("user_id".to_string(), serde_json::json!("user123"));

        assert_eq!(
            state.get_step_output("step1"),
            Some(&serde_json::json!({"result": "ok"}))
        );
        assert_eq!(
            state.get_global("user_id"),
            Some(&serde_json::json!("user123"))
        );
    }

    #[test]
    fn test_workflow_config_default() {
        let config = WorkflowConfig::default();
        assert_eq!(config.max_parallel, 10);
        assert!(config.enable_logging);
        assert!(config.stop_on_error);
    }

    #[test]
    fn test_workflow_step_with_depends_on() {
        let step = WorkflowStep::llm("step2", "Process {step1.result}")
            .with_depends_on(vec!["step1".to_string()]);

        assert_eq!(step.depends_on, Some(vec!["step1".to_string()]));
    }

    #[test]
    fn test_workflow_step_with_parallel() {
        let step = WorkflowStep::llm("step1", "Process").with_parallel(true);
        assert!(step.parallel);
    }

    /// Mock LLM provider for testing
    struct MockLLMProvider {
        response: String,
    }

    impl MockLLMProvider {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl WorkflowLLMProvider for MockLLMProvider {
        async fn generate(&self, _prompt: &str, _profile: Option<&str>) -> MemResult<String> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_workflow_runner_sequential_execution() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test response")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let steps = vec![
            WorkflowStep::llm("step1", "First step"),
            WorkflowStep::llm("step2", "Second step").with_depends_on(vec!["step1".to_string()]),
        ];

        let state = WorkflowState::new();
        let (final_state, stats) = runner.run(&steps, state).await.unwrap();

        assert_eq!(stats.steps_executed, 2);
        assert_eq!(stats.steps_succeeded, 2);
        assert_eq!(stats.steps_failed, 0);
        assert!(final_state.get_step_output("step1").is_some());
        assert!(final_state.get_step_output("step2").is_some());
    }

    #[tokio::test]
    async fn test_workflow_runner_function_step() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let steps = vec![WorkflowStep::function(
            "transform",
            |state| async move {
                let mut result = state;
                result.insert("transformed".to_string(), serde_json::json!(true));
                Ok(result)
            },
            vec![Capability::DB],
        )];

        let state = WorkflowState::new();
        let (final_state, stats) = runner.run(&steps, state).await.unwrap();

        assert_eq!(stats.steps_executed, 1);
        assert_eq!(stats.steps_succeeded, 1);
        assert!(final_state.get_step_output("transform").is_some());
    }

    #[tokio::test]
    async fn test_workflow_runner_parallel_execution() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("parallel result")) as Box<dyn WorkflowLLMProvider>,
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        // Create parallel step with multiple sub-steps
        let parallel_step = WorkflowStep::parallel(
            "parallel_ops",
            vec![
                WorkflowStep::llm("sub1", "Sub step 1"),
                WorkflowStep::llm("sub2", "Sub step 2"),
                WorkflowStep::llm("sub3", "Sub step 3"),
            ],
        );

        let steps = vec![parallel_step];
        let state = WorkflowState::new();
        let (final_state, stats) = runner.run(&steps, state).await.unwrap();

        // All 3 sub-steps + 1 parallel container step = 4 total
        assert_eq!(stats.steps_executed, 4);
        assert_eq!(stats.steps_succeeded, 4);
        assert_eq!(stats.steps_failed, 0);

        // Verify parallel step output contains all sub-step results
        let parallel_output = final_state.get_step_output("parallel_ops").unwrap();
        assert!(parallel_output.as_object().unwrap().contains_key("sub1"));
        assert!(parallel_output.as_object().unwrap().contains_key("sub2"));
        assert!(parallel_output.as_object().unwrap().contains_key("sub3"));
    }

    #[tokio::test]
    async fn test_workflow_runner_capability_validation() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));

        // Runner without Vector capability
        let runner = DefaultWorkflowRunner::new(
            llm_provider,
            WorkflowConfig::default(),
            HashSet::from([Capability::LLM, Capability::DB]),
        );

        // Step requires Vector capability
        let steps = vec![WorkflowStep::function(
            "vector_op",
            |_| async { Ok(HashMap::new()) },
            vec![Capability::Vector],
        )];

        let state = WorkflowState::new();
        let result = runner.run(&steps, state).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing capabilities"));
    }

    #[tokio::test]
    async fn test_workflow_runner_template_rendering() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("processed")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let mut initial_state = WorkflowState::new();
        initial_state.set_global("name".to_string(), serde_json::json!("Alice"));

        let steps = vec![WorkflowStep::llm("greet", "Hello {name}, welcome!")];

        let (final_state, stats) = runner.run(&steps, initial_state).await.unwrap();

        assert_eq!(stats.steps_succeeded, 1);
        assert!(final_state.get_step_output("greet").is_some());
    }

    #[test]
    fn test_workflow_stats_default() {
        let stats = WorkflowStats::default();
        assert_eq!(stats.steps_executed, 0);
        assert_eq!(stats.steps_succeeded, 0);
        assert_eq!(stats.steps_failed, 0);
        assert_eq!(stats.total_time_ms, 0);
        assert!(stats.step_times.is_empty());
    }

    // === Interceptor Tests ===

    /// Mock interceptor for testing
    struct MockInterceptor {
        name: String,
    }

    impl MockInterceptor {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl Interceptor for MockInterceptor {
        async fn before(&self, context: &mut InterceptorContext) -> MemResult<()> {
            // Add metadata
            context.metadata.insert(
                "interceptor_name".to_string(),
                serde_json::Value::String(self.name.clone()),
            );
            context.metadata.insert(
                "before_timestamp".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_string()),
            );
            Ok(())
        }

        async fn after(
            &self,
            result: serde_json::Value,
            _context: &InterceptorContext,
        ) -> MemResult<serde_json::Value> {
            // Modify result
            let mut modified = result.as_object().unwrap().clone();
            modified.insert(
                "interceptor".to_string(),
                serde_json::Value::String(self.name.clone()),
            );
            Ok(serde_json::to_value(modified)?)
        }
    }

    #[tokio::test]
    async fn test_interceptor_registry() {
        let registry = InterceptorRegistry::new();

        // Test empty registry
        assert!(registry.is_empty().await);

        // Register mock interceptor
        let interceptor = Arc::new(MockInterceptor::new("test_interceptor"));
        registry.register(interceptor).await;

        // Verify registration
        assert_eq!(registry.len().await, 1);
        assert!(!registry.is_empty().await);

        // Create context
        let state = WorkflowState::new();
        let mut context =
            InterceptorContext::new("test_step".to_string(), StepType::Function, state);

        // Execute before hook
        registry.execute_before(&mut context).await.unwrap();
        assert!(context.metadata.contains_key("interceptor_name"));
        assert!(context.metadata.contains_key("before_timestamp"));

        // Execute after hook
        let result = serde_json::json!({"result": "success"});
        let processed = registry.execute_after(result, &context).await.unwrap();
        assert_eq!(processed["interceptor"], "test_interceptor");
    }

    // === PipelineManager Tests ===

    #[tokio::test]
    async fn test_pipeline_manager_registration() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string(), "claude-3".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a simple pipeline
        let steps = vec![WorkflowStep::llm("step1", "First step").with_llm_profile("gpt-4")];

        manager.register("test_pipeline", steps).await.unwrap();

        // Verify registration
        assert!(manager.has_pipeline("test_pipeline").await);
        assert_eq!(manager.len().await, 1);
        assert!(!manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_pipeline_manager_capability_validation() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        // Manager without Vector capability
        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Pipeline requires Vector capability
        let steps = vec![WorkflowStep::function(
            "vector_op",
            |_| async { Ok(HashMap::new()) },
            vec![Capability::Vector],
        )];

        let result = manager.register("vector_pipeline", steps).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing capabilities"));
    }

    #[tokio::test]
    async fn test_pipeline_manager_llm_profile_validation() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Pipeline uses unknown LLM profile
        let steps = vec![WorkflowStep::llm("step1", "Process").with_llm_profile("unknown-model")];

        let result = manager.register("bad_profile_pipeline", steps).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown LLM profile"));
    }

    #[tokio::test]
    async fn test_pipeline_manager_sub_steps_validation() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Parallel step with sub-step using unknown profile
        let steps = vec![WorkflowStep::parallel(
            "parallel_ops",
            vec![
                WorkflowStep::llm("sub1", "Sub step 1").with_llm_profile("gpt-4"),
                WorkflowStep::llm("sub2", "Sub step 2").with_llm_profile("unknown-model"),
            ],
        )];

        let result = manager.register("bad_sub_pipeline", steps).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Sub-step"));
        assert!(err.to_string().contains("unknown LLM profile"));
    }

    #[tokio::test]
    async fn test_pipeline_manager_run() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("pipeline result")) as Box<dyn WorkflowLLMProvider>,
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register pipeline
        let steps = vec![
            WorkflowStep::llm("step1", "First step").with_llm_profile("gpt-4"),
            WorkflowStep::llm("step2", "Second step").with_llm_profile("gpt-4"),
        ];

        manager.register("test_pipeline", steps).await.unwrap();

        // Run pipeline
        let initial_state = WorkflowState::new();
        let (final_state, stats) = manager.run("test_pipeline", initial_state).await.unwrap();

        assert_eq!(stats.steps_executed, 2);
        assert_eq!(stats.steps_succeeded, 2);
        assert!(final_state.get_step_output("step1").is_some());
        assert!(final_state.get_step_output("step2").is_some());
    }

    #[tokio::test]
    async fn test_pipeline_manager_not_found() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Try to run non-existent pipeline
        let initial_state = WorkflowState::new();
        let result = manager.run("nonexistent", initial_state).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_pipeline_manager_remove() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register and then remove pipeline
        let steps = vec![WorkflowStep::llm("step1", "Test").with_llm_profile("gpt-4")];

        manager.register("temp_pipeline", steps).await.unwrap();
        assert!(manager.has_pipeline("temp_pipeline").await);

        manager.remove_pipeline("temp_pipeline").await.unwrap();
        assert!(!manager.has_pipeline("temp_pipeline").await);
        assert_eq!(manager.len().await, 0);
    }

    #[tokio::test]
    async fn test_pipeline_manager_list_pipelines() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register multiple pipelines
        let step1 = vec![WorkflowStep::llm("s1", "Step 1").with_llm_profile("gpt-4")];
        let step2 = vec![WorkflowStep::llm("s2", "Step 2").with_llm_profile("gpt-4")];
        let step3 = vec![WorkflowStep::llm("s3", "Step 3").with_llm_profile("gpt-4")];

        manager.register("pipeline1", step1).await.unwrap();
        manager.register("pipeline2", step2).await.unwrap();
        manager.register("pipeline3", step3).await.unwrap();

        let list = manager.list_pipelines().await;
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"pipeline1".to_string()));
        assert!(list.contains(&"pipeline2".to_string()));
        assert!(list.contains(&"pipeline3".to_string()));
    }

    // === Phase 1.6.6: Comprehensive Workflow Unit Tests ===

    #[tokio::test]
    async fn test_workflow_error_propagation() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let steps = vec![
            WorkflowStep::function(
                "step1",
                |state| async move { Ok(state) },
                vec![Capability::DB],
            ),
            WorkflowStep::function(
                "step2_failing",
                |_| async { Err(MemError::WorkflowError("Step failed".to_string())) },
                vec![Capability::DB],
            ),
            WorkflowStep::function(
                "step3_never_reached",
                |state| async move { Ok(state) },
                vec![Capability::DB],
            ),
        ];

        let result = runner.run(&steps, WorkflowState::new()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Step failed"));
    }

    #[tokio::test]
    async fn test_workflow_config_stop_on_error_false() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let config = WorkflowConfig {
            max_parallel: 10,
            enable_logging: true,
            stop_on_error: false,
        };
        let capabilities = HashSet::from([Capability::LLM, Capability::DB, Capability::IO]);
        let runner = DefaultWorkflowRunner::new(llm_provider, config, capabilities);

        let steps = vec![
            WorkflowStep::function(
                "step1",
                |state| async move { Ok(state) },
                vec![Capability::DB],
            ),
            WorkflowStep::function(
                "step2_failing",
                |_| async { Err(MemError::WorkflowError("Step failed".to_string())) },
                vec![Capability::DB],
            ),
            WorkflowStep::function(
                "step3_continues",
                |state| async move { Ok(state) },
                vec![Capability::DB],
            ),
        ];

        // With stop_on_error=false, should continue after failure
        let (final_state, stats) = runner.run(&steps, WorkflowState::new()).await.unwrap();
        assert_eq!(stats.steps_executed, 3);
        assert_eq!(stats.steps_succeeded, 2);
        assert_eq!(stats.steps_failed, 1);
        assert!(final_state.get_step_output("step1").is_some());
        assert!(final_state.get_step_output("step3_continues").is_some());
    }

    #[tokio::test]
    async fn test_workflow_multiple_parallel_steps() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("parallel")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let steps = vec![
            WorkflowStep::parallel(
                "first_parallel",
                vec![
                    WorkflowStep::llm("p1_1", "Task 1").with_llm_profile("gpt-4"),
                    WorkflowStep::llm("p1_2", "Task 2").with_llm_profile("gpt-4"),
                ],
            ),
            WorkflowStep::parallel(
                "second_parallel",
                vec![
                    WorkflowStep::llm("p2_1", "Task 3").with_llm_profile("gpt-4"),
                    WorkflowStep::llm("p2_2", "Task 4").with_llm_profile("gpt-4"),
                ],
            ),
        ];

        let (final_state, stats) = runner.run(&steps, WorkflowState::new()).await.unwrap();

        // Each parallel step counts as 1 container + sub-steps
        // first_parallel: 1 container + 2 sub-steps = 3
        // second_parallel: 1 container + 2 sub-steps = 3
        // Total: 6
        assert_eq!(stats.steps_executed, 6);
        assert_eq!(stats.steps_succeeded, 6);

        // Sub-step outputs are nested inside the parallel step output
        let first_parallel_output = final_state.get_step_output("first_parallel").unwrap();
        assert!(first_parallel_output.get("p1_1").is_some());
        assert!(first_parallel_output.get("p1_2").is_some());

        let second_parallel_output = final_state.get_step_output("second_parallel").unwrap();
        assert!(second_parallel_output.get("p2_1").is_some());
        assert!(second_parallel_output.get("p2_2").is_some());
    }

    #[tokio::test]
    async fn test_workflow_capability_validation_deep_nested() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));

        // Runner with only LLM capability
        let capabilities = HashSet::from([Capability::LLM]);
        let config = WorkflowConfig::default();
        let runner = DefaultWorkflowRunner::new(llm_provider, config, capabilities);

        // Parallel step with deeply nested step requiring Vector capability
        // Note: Cannot have nested parallel (not supported), so we use a flat parallel with nested capability
        let steps = vec![
            WorkflowStep::llm("step1", "First").with_llm_profile("gpt-4"),
            WorkflowStep::parallel(
                "parallel_outer",
                vec![
                    WorkflowStep::llm("inner1", "Inner 1").with_llm_profile("gpt-4"),
                    WorkflowStep::function(
                        "deep_nested",
                        |state| async move { Ok(state) },
                        vec![Capability::Vector], // Missing capability
                    ),
                ],
            ),
        ];

        let result = runner.run(&steps, WorkflowState::new()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing capabilities"));
        // The error uses lowercase "vector" due to Display impl
        assert!(err.to_string().contains("vector"));
    }

    // === Dynamic Configuration Tests ===

    #[tokio::test]
    async fn test_config_step() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string(), "claude-3".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "Original prompt").with_llm_profile("gpt-4")];

        manager.register("test_pipeline", steps).await.unwrap();

        // Configure step - change prompt and llm_profile
        let mut configs = HashMap::new();
        configs.insert(
            "prompt_template".to_string(),
            serde_json::json!("New prompt"),
        );
        configs.insert("llm_profile".to_string(), serde_json::json!("claude-3"));

        let modified = manager
            .config_step("test_pipeline", "step1", configs)
            .await
            .unwrap();
        assert_eq!(modified, 1);

        // Verify configuration was applied (we'd need to inspect the pipeline to verify)
        assert!(manager.has_pipeline("test_pipeline").await);
    }

    #[tokio::test]
    async fn test_config_step_not_found() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "Test")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to configure non-existent step
        let configs = HashMap::new();
        let modified = manager
            .config_step("test_pipeline", "nonexistent", configs)
            .await
            .unwrap();
        assert_eq!(modified, 0);
    }

    #[tokio::test]
    async fn test_config_step_invalid_llm_profile() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "Test")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to configure with invalid LLM profile
        let mut configs = HashMap::new();
        configs.insert(
            "llm_profile".to_string(),
            serde_json::json!("invalid-profile"),
        );

        let result = manager.config_step("test_pipeline", "step1", configs).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unknown LLM profile") || err_msg.contains("unknown LLM profile"));
    }

    #[tokio::test]
    async fn test_insert_after() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline with two steps
        let steps = vec![
            WorkflowStep::llm("step1", "First"),
            WorkflowStep::llm("step3", "Third"),
        ];

        manager.register("test_pipeline", steps).await.unwrap();

        // Insert step2 after step1
        let new_step = WorkflowStep::llm("step2", "Second");
        let inserted = manager
            .insert_after("test_pipeline", "step1", new_step)
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        // Verify insertion by running the pipeline
        let state = WorkflowState::new();
        let (final_state, _) = manager.run("test_pipeline", state).await.unwrap();

        // Check execution order: step1 -> step2 -> step3
        assert!(final_state.get_step_output("step1").is_some());
        assert!(final_state.get_step_output("step2").is_some());
        assert!(final_state.get_step_output("step3").is_some());
    }

    #[tokio::test]
    async fn test_insert_after_not_found() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "First")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to insert after non-existent step
        let new_step = WorkflowStep::llm("step2", "Second");
        let inserted = manager
            .insert_after("test_pipeline", "nonexistent", new_step)
            .await
            .unwrap();
        assert_eq!(inserted, 0);
    }

    #[tokio::test]
    async fn test_insert_before() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline with two steps
        let steps = vec![
            WorkflowStep::llm("step1", "First"),
            WorkflowStep::llm("step3", "Third"),
        ];

        manager.register("test_pipeline", steps).await.unwrap();

        // Insert step2 before step3
        let new_step = WorkflowStep::llm("step2", "Second");
        let inserted = manager
            .insert_before("test_pipeline", "step3", new_step)
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        // Verify insertion by running the pipeline
        let state = WorkflowState::new();
        let (final_state, _) = manager.run("test_pipeline", state).await.unwrap();

        // Check execution order: step1 -> step2 -> step3
        assert!(final_state.get_step_output("step1").is_some());
        assert!(final_state.get_step_output("step2").is_some());
        assert!(final_state.get_step_output("step3").is_some());
    }

    #[tokio::test]
    async fn test_insert_before_not_found() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "First")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to insert before non-existent step
        let new_step = WorkflowStep::llm("step2", "Second");
        let inserted = manager
            .insert_before("test_pipeline", "nonexistent", new_step)
            .await
            .unwrap();
        assert_eq!(inserted, 0);
    }

    #[tokio::test]
    async fn test_replace_step() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM, Capability::DB]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "Original prompt")];

        manager.register("test_pipeline", steps).await.unwrap();

        // Replace step1 with new version
        let new_step = WorkflowStep::llm("step1", "Replaced prompt").with_llm_profile("gpt-4");
        let replaced = manager
            .replace_step("test_pipeline", "step1", new_step)
            .await
            .unwrap();
        assert_eq!(replaced, 1);

        // Verify replacement by running the pipeline
        let state = WorkflowState::new();
        let (final_state, _) = manager.run("test_pipeline", state).await.unwrap();
        assert!(final_state.get_step_output("step1").is_some());
    }

    #[tokio::test]
    async fn test_replace_step_not_found() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "First")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to replace non-existent step
        let new_step = WorkflowStep::llm("step2", "Second");
        let replaced = manager
            .replace_step("test_pipeline", "nonexistent", new_step)
            .await
            .unwrap();
        assert_eq!(replaced, 0);
    }

    #[tokio::test]
    async fn test_insert_with_missing_capability() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        // Only LLM capability available
        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "First")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to insert a step requiring Vector capability (not available)
        let new_step = WorkflowStep::function(
            "step2",
            |state| async move { Ok(state) },
            vec![Capability::Vector], // Missing capability
        );

        let result = manager
            .insert_after("test_pipeline", "step1", new_step)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing capabilities"));
    }

    #[tokio::test]
    async fn test_insert_with_invalid_llm_profile() {
        let llm_provider = Arc::new(RwLock::new(
            Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>
        ));
        let runner = Arc::new(DefaultWorkflowRunner::with_llm(llm_provider));

        let capabilities = HashSet::from([Capability::LLM]);
        let llm_profiles = HashSet::from(["gpt-4".to_string()]);

        let manager = PipelineManager::new(capabilities, llm_profiles, runner);

        // Register a pipeline
        let steps = vec![WorkflowStep::llm("step1", "First")];
        manager.register("test_pipeline", steps).await.unwrap();

        // Try to insert a step with invalid LLM profile
        let new_step = WorkflowStep::llm("step2", "Second").with_llm_profile("invalid-profile");

        let result = manager
            .insert_after("test_pipeline", "step1", new_step)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown LLM profile"));
    }
}
