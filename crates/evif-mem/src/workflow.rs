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
    dyn Fn(HashMap<String, serde_json::Value>) -> Pin<Box<dyn Future<Output = MemResult<HashMap<String, serde_json::Value>>> + Send>>
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
    pub fn function<F, Fut>(step_id: impl Into<String>, func: F, capabilities: Vec<Capability>) -> Self
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
        let func = step
            .function
            .as_ref()
            .ok_or_else(|| MemError::WorkflowError(format!("Step '{}' has no function", step.step_id)))?;

        let state_map = state.step_outputs.clone();
        let mut result: HashMap<String, serde_json::Value> = func(state_map).await?;

        // Merge result into state
        if let Some(global) = result.remove("global") {
            let mut state = state.clone();
            if let Ok(global_map) = serde_json::from_value::<HashMap<String, serde_json::Value>>(global) {
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
        let template = step
            .prompt_template
            .as_ref()
            .ok_or_else(|| MemError::WorkflowError(format!("Step '{}' has no prompt template", step.step_id)))?;

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
        let sub_steps = step
            .sub_steps
            .as_ref()
            .ok_or_else(|| MemError::WorkflowError(format!("Parallel step '{}' has no sub-steps", step.step_id)))?;

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
                            MemError::WorkflowError(format!("Step '{}' has no prompt template", step_id))
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
                    StepType::Parallel => {
                        Err(MemError::WorkflowError(
                            "Nested parallel steps are not supported".to_string(),
                        ))
                    }
                };

                let elapsed = start.elapsed().as_millis() as u64;
                Ok::<_, MemError>((step_id, result, elapsed))
            });

            handles.push(handle);
        }

        // Collect results from all concurrent tasks
        let mut results = HashMap::new();
        for handle in handles {
            let (step_id, result, elapsed) = handle.await
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
        let step = WorkflowStep::llm("extract", "Extract memories from: {text}")
            .with_llm_profile("gpt-4");

        assert_eq!(step.step_id, "extract");
        assert_eq!(step.step_type, StepType::LLM);
        assert!(step.capabilities.contains(&Capability::LLM));
        assert_eq!(step.prompt_template, Some("Extract memories from: {text}".to_string()));
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
        let llm_provider = Arc::new(RwLock::new(Box::new(MockLLMProvider::new("test response")) as Box<dyn WorkflowLLMProvider>));
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
        let llm_provider = Arc::new(RwLock::new(Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let steps = vec![
            WorkflowStep::function(
                "transform",
                |state| async move {
                    let mut result = state;
                    result.insert("transformed".to_string(), serde_json::json!(true));
                    Ok(result)
                },
                vec![Capability::DB],
            ),
        ];

        let state = WorkflowState::new();
        let (final_state, stats) = runner.run(&steps, state).await.unwrap();

        assert_eq!(stats.steps_executed, 1);
        assert_eq!(stats.steps_succeeded, 1);
        assert!(final_state.get_step_output("transform").is_some());
    }

    #[tokio::test]
    async fn test_workflow_runner_parallel_execution() {
        let llm_provider = Arc::new(RwLock::new(Box::new(MockLLMProvider::new("parallel result")) as Box<dyn WorkflowLLMProvider>));
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
        let llm_provider = Arc::new(RwLock::new(Box::new(MockLLMProvider::new("test")) as Box<dyn WorkflowLLMProvider>));

        // Runner without Vector capability
        let runner = DefaultWorkflowRunner::new(
            llm_provider,
            WorkflowConfig::default(),
            HashSet::from([Capability::LLM, Capability::DB]),
        );

        // Step requires Vector capability
        let steps = vec![
            WorkflowStep::function(
                "vector_op",
                |_| async { Ok(HashMap::new()) },
                vec![Capability::Vector],
            ),
        ];

        let state = WorkflowState::new();
        let result = runner.run(&steps, state).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing capabilities"));
    }

    #[tokio::test]
    async fn test_workflow_runner_template_rendering() {
        let llm_provider = Arc::new(RwLock::new(Box::new(MockLLMProvider::new("processed")) as Box<dyn WorkflowLLMProvider>));
        let runner = DefaultWorkflowRunner::with_llm(llm_provider);

        let mut initial_state = WorkflowState::new();
        initial_state.set_global("name".to_string(), serde_json::json!("Alice"));

        let steps = vec![
            WorkflowStep::llm("greet", "Hello {name}, welcome!"),
        ];

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
}
