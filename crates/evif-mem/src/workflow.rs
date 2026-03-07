//! Workflow system for configurable memory pipelines
//!
//! This module provides a flexible workflow engine that allows dynamic configuration
//! of memory processing pipelines, inspired by memU's workflow system.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
}
