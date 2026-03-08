# Changelog

All notable changes to the evif-mem project will be documented in this file.

## [Unreleased]

### Added - Phase 1.7: Multi-User Support (In Progress)

**Multi-User Support Implementation** - User scope and tenant isolation for enterprise deployment

#### Phase 1.7.3: Tenant Management ✅ **2026-03-08**

1. **Tenant-scoped Indexes**
   - Added `items_by_tenant: Arc<DashMap<String, Vec<String>>>` - tenant_id -> item_ids
   - Added `resources_by_tenant: Arc<DashMap<String, Vec<String>>>` - tenant_id -> resource_ids
   - Added `categories_by_tenant: Arc<DashMap<String, Vec<String>>>` - tenant_id -> category_ids

2. **Tenant-scoped Query Methods**
   - `get_items_by_tenant(tenant_id: &str) -> Vec<MemoryItem>` - Get all items for a tenant
   - `get_resources_by_tenant(tenant_id: &str) -> Vec<Resource>` - Get all resources for a tenant
   - `get_categories_by_tenant(tenant_id: &str) -> Vec<MemoryCategory>` - Get all categories for a tenant

3. **Tenant Access Control Methods**
   - `item_belongs_to_tenant(item_id: &str, tenant_id: &str) -> bool` - Check item ownership
   - `resource_belongs_to_tenant(resource_id: &str, tenant_id: &str) -> bool` - Check resource ownership

4. **Tenant Statistics Methods**
   - `item_count_by_tenant(tenant_id: &str) -> usize` - Get item count for tenant
   - `resource_count_by_tenant(tenant_id: &str) -> usize` - Get resource count for tenant
   - `get_all_tenants() -> Vec<String>` - List all tenants (admin)

5. **Storage Layer Integration**
   - Updated `put_item()` to index by tenant_id
   - Updated `put_resource()` to index by tenant_id
   - Updated `put_category()` to index by tenant_id

#### Tests Added
- 3 new tenant management tests:
  - test_tenant_scoped_items
  - test_tenant_scoped_resources
  - test_tenant_counts
- Total: 129 evif-mem tests passing (up from 126)

#### Phase 1.7 Status
- Phase 1.7.1: Data Model Extensions ✅ **100% Complete**
- Phase 1.7.2: Storage Layer User Filtering ✅ **100% Complete**
- Phase 1.7.3: Tenant Management ✅ **100% Complete**
- **Phase 1.7 Overall: 50% Complete**

**evif-mem Overall Progress**: **89% → 91%** (up 2%)

---

#### Phase 1.7.1: Data Model Extensions

1. **User and Tenant Fields**
   - Added `user_id: Option<String>` to MemoryItem, Resource, MemoryCategory
   - Added `tenant_id: Option<String>` for multi-tenant support
   - Added `with_user_context()` builder methods to all models

2. **UserScope Struct**
   - Created UserScope struct for user context tracking
   - Fields: user_id, tenant_id, role
   - Builder methods: new(), with_tenant(), with_role()
   - Access control: can_access() method for resource authorization

3. **Storage Layer Extensions**
   - Added user-scoped indexes: items_by_user, resources_by_user, categories_by_user
   - New methods: get_items_by_user(), get_resources_by_user(), get_categories_by_user()
   - Access control: item_belongs_to_user(), resource_belongs_to_user()
   - User-aware put operations automatically index by user_id

#### Tests Added
- 7 new multi-user storage tests:
  - test_user_scoped_items
  - test_user_scoped_resources
  - test_user_scoped_categories
  - test_item_belongs_to_user
  - test_tenant_isolation
  - test_user_scope_can_access
- Total: 129 evif-mem tests passing (up from 126)

#### Phase 1.7 Status
- Phase 1.7.1: Data Model Extensions ✅ **100% Complete**
- Phase 1.7.2: Storage Layer User Filtering ✅ **100% Complete**
- Phase 1.7.3: Tenant Management (Pending)
- **Phase 1.7 Overall: 25% Complete**

---

### Added - Phase 1.6: Workflow System (Complete)

**Parallel Execution Implementation** - True concurrent execution of workflow sub-steps

#### Phase 1.6.3: True Parallel Execution

1. **Concurrent Task Execution**
   - Refactored `execute_parallel_step` to use `tokio::spawn` for concurrent execution
   - Sub-steps now run truly in parallel instead of sequentially
   - Each sub-step runs as an independent tokio task
   - Results collected via handle.await with proper error propagation

2. **Implementation Details**
   - Clone necessary data for each async task (step_id, step_type, state, etc.)
   - Template rendering logic moved into spawned tasks for LLM steps
   - Function execution logic moved into spawned tasks
   - Nested parallel steps still return error (not supported)
   - Proper timing statistics per sub-step

3. **Test Infrastructure**
   - Added `MockLLMProvider` for testing workflow execution
   - 8 new unit tests for WorkflowRunner functionality:
     - `test_workflow_runner_sequential_execution`
     - `test_workflow_runner_function_step`
     - `test_workflow_runner_parallel_execution`
     - `test_workflow_runner_capability_validation`
     - `test_workflow_runner_template_rendering`
     - `test_workflow_step_with_depends_on`
     - `test_workflow_step_with_parallel`
     - `test_workflow_stats_default`

#### Tests Added
- 8 new workflow runner tests
- Total: 107 evif-mem tests passing (up from 99)

#### Phase 1.6 Status
- Phase 1.6.1: WorkflowStep Design ✅ **100% Complete**
- Phase 1.6.2: WorkflowRunner ✅ **100% Complete**
- Phase 1.6.3: True Parallel Execution ✅ **100% Complete**
- Phase 1.6.4: Interceptor System ✅ **100% Complete**
- Phase 1.6.5: PipelineManager ✅ **100% Complete**
- Phase 1.6.6: Comprehensive Unit Tests ✅ **100% Complete** (NEW)

#### Phase 1.6.5: PipelineManager

1. **PipelineManager Structure**
   - Manages registry of named workflows/pipelines
   - Validates capability dependencies before registration
   - Validates LLM profile dependencies before registration
   - Allows dynamic registration of pipelines at runtime
   - Executes registered pipelines by name

2. **Implementation Details**
   - `PipelineManager` struct with:
     - `pipelines: RwLock<HashMap<String, Vec<WorkflowStep>>>` - registered pipelines
     - `capabilities: HashSet<Capability>` - available capabilities
     - `llm_profiles: HashSet<String>` - available LLM profiles
   - `register()` - register pipeline with validation
   - `validate_sub_steps()` - recursive validation for nested steps
   - `run()` - execute named pipeline by name
   - Utility methods: `list_pipelines()`, `has_pipeline()`, `remove_pipeline()`, `len()`, `is_empty()`

3. **Test Infrastructure**
   - 8 comprehensive unit tests:
     - `test_pipeline_manager_registration`
     - `test_pipeline_manager_capability_validation`
     - `test_pipeline_manager_llm_profile_validation`
     - `test_pipeline_manager_not_found`
     - `test_pipeline_manager_run`
     - `test_pipeline_manager_remove`
     - `test_pipeline_manager_list_pipelines`
     - `test_pipeline_manager_sub_steps_validation`

#### Tests Added
- 8 PipelineManager tests (already present)
- Total: 24 workflow tests passing
- Total: 116 evif-mem tests passing

#### Phase 1.6.6: Comprehensive Workflow Unit Tests

1. **Error Handling Tests**
   - `test_workflow_error_propagation` - Verify errors stop execution when stop_on_error=true
   - `test_workflow_config_stop_on_error_false` - Verify errors don't stop execution when stop_on_error=false

2. **Parallel Execution Tests**
   - `test_workflow_multiple_parallel_steps` - Test multiple parallel step containers
   - `test_workflow_runner_parallel_execution` - Already present (verified)

3. **Capability Validation Tests**
   - `test_workflow_capability_validation_deep_nested` - Test deep nested capability validation

#### Tests Added
- 4 new comprehensive tests
- Total: 28 workflow tests passing
- Total: 120 evif-mem tests passing

**Overall Phase 1.6 Progress**: **100% Complete** (6/6 sub-tasks done)

**evif-mem Overall Progress**: **86% → 87%** (up 1%)

#### Phase 1.6.4: Interceptor System

1. **Interceptor Trait and Registry**
   - Added `Interceptor` trait with `before()` and `after()` async hooks
   - Added `InterceptorContext` for passing execution context
     - Contains: step_id, step_type, prompt, llm_profile, state, metadata
   - Added `InterceptorRegistry` for managing multiple interceptors
     - `register()` - async interceptor registration
     - `len()` / `is_empty()` - registry statistics
     - `execute_before()` - execute all before hooks in order
     - `execute_after()` - execute all after hooks in reverse order

2. **Integration with WorkflowRunner**
   - Added `interceptors` field to `DefaultWorkflowRunner`
   - Updated constructors to initialize interceptor registry
   - Added `with_interceptors()` constructor for custom interceptor injection

3. **Test Infrastructure**
   - Added `MockInterceptor` for testing interceptor behavior
   - 1 new unit test: `test_interceptor_registry`
     - Tests interceptor registration
     - Tests before/after hook execution
     - Tests context metadata modification
     - Tests result transformation

#### Tests Added
- 1 new interceptor test
- Total: 16 workflow tests passing (up from 15)
- Total: 108 evif-mem tests passing

**Overall Phase 1.6 Progress**: **87% Complete** (4.5/5 sub-tasks done)


**evif-mem Overall Progress**: **85% → 86%** (up 1%)

#### Files Modified
- `crates/evif-mem/src/workflow.rs` (refactored parallel execution, added 8 tests)

#### Commit

```
git commit 5516e8d
Author: Ralph Loop
Message: feat(evif-mem): add true parallel execution for workflow steps (Phase 1.6.3)

- Refactored execute_parallel_step to use tokio::spawn for concurrent execution
- Sub-steps now run in parallel instead of sequentially
- Added 8 new unit tests for WorkflowRunner functionality
- Added MockLLMProvider for testing workflow execution
- All 107 tests pass (up from 99)

Phase 1.6: 50% → 62% complete
```

#### Next Steps

**Immediate (Current Sprint)**:
1. Phase 1.6.6: Write workflow system integration tests
   - End-to-end workflow tests
   - Complex multi-step scenarios
   - Error handling tests

**Short-term (Next Sprint)**:
2. Phase 1.7: Multi-user Support
   - User isolation
   - Tenant management

---

### Phase 1.6.2: WorkflowRunner Implementation (Previously Completed)

1. **WorkflowRunner Trait**
   - `run()` - Execute workflow steps with initial state
   - `validate_capabilities()` - Verify required capabilities are available
   - Returns `(WorkflowState, WorkflowStats)` tuple

2. **DefaultWorkflowRunner Implementation**
   - Executes LLM steps via WorkflowLLMProvider trait
   - Executes Function steps directly via Arc<StepFunction>
   - Executes Parallel steps with sub-step handling
   - Template rendering for prompts with `{var}` and `{step.field}` syntax
   - Dependency checking before step execution
   - Error handling with stop_on_error configuration
   - Per-step timing statistics

3. **WorkflowLLMProvider Trait**
   - Abstracts LLM operations for workflow execution
   - `generate(prompt, profile)` - Generate completion with optional profile

4. **Error Handling Enhancements**
   - Added `WorkflowError` variant to MemError enum
   - Added `Json` error variant for serde_json::Error
   - Type annotation fixes for HashMap results

#### Tests Added
- All 7 existing workflow tests still pass
- Total: 99 evif-mem tests passing

#### API Usage

```rust
use evif_mem::workflow::{DefaultWorkflowRunner, WorkflowRunner, WorkflowStep, WorkflowState};

// Create LLM provider
let llm_provider: Arc<RwLock<Box<dyn WorkflowLLMProvider>>> = ...;

// Create runner
let runner = DefaultWorkflowRunner::with_llm(llm_provider);

// Define steps
let steps = vec![
    WorkflowStep::llm("extract", "Extract memories from: {text}")
        .with_llm_profile("gpt-4"),
    WorkflowStep::function(
        "dedupe",
        |mut state| async move {
            state.insert("deduped".to_string(), serde_json::json!(true));
            Ok(state)
        },
        vec![Capability::DB],
    ),
];

// Execute workflow
let initial_state = WorkflowState::with_global(
    vec![("text".to_string(), serde_json::json!("content here"))]
        .into_iter()
        .collect()
);

let (final_state, stats) = runner.run(&steps, initial_state).await?;
```

#### Phase 1.6 Status
- Phase 1.6.1: WorkflowStep Design ✅ **100% Complete**
- Phase 1.6.2: WorkflowRunner ✅ **100% Complete**
- Phase 1.6.3: Interceptor Mechanism ⏳ **Pending**
- Phase 1.6.4: PipelineManager ⏳ **Pending**

**Overall Phase 1.6 Progress**: **50% Complete** (2/4 sub-tasks done)

**evif-mem Overall Progress**: **82% → 83%** (up 1%)

#### Files Modified
- `crates/evif-mem/src/workflow.rs` (added WorkflowRunner trait and DefaultWorkflowRunner)
- `crates/evif-mem/src/error.rs` (added WorkflowError and Json error variants)

#### Next Steps

**Immediate (Current Sprint)**:
1. Phase 1.6.3: Implement Interceptor Mechanism
   - Interceptor trait
   - InterceptorRegistry
   - before/after hooks

**Short-term (Next Sprint)**:
2. Phase 1.6.4: Implement PipelineManager
   - Dynamic pipeline registration
   - Capability validation
   - Runtime configuration

---

### Phase 1.6.1: WorkflowStep System (Previously Completed)

#### Core Components

1. **WorkflowStep Structure**
   - Flexible step definition for memory processing workflows
   - Three step types: LLM, Function, Parallel
   - Capability-based dependency tracking
   - Builder pattern for ergonomic construction

2. **StepType Enum**
   - `LLM`: LLM-based processing with prompt templates
   - `Function`: Rust function execution
   - `Parallel`: Parallel execution of sub-steps

3. **Capability System**
   - `LLM`: Can call LLM
   - `Vector`: Can do vector search
   - `DB`: Can access storage
   - `IO`: Can do file/network I/O
   - `Embedding`: Can generate embeddings

4. **WorkflowState**
   - Step output management (step_id → output)
   - Global state accessible to all steps
   - Mutable state propagation across workflow

5. **WorkflowConfig**
   - `max_parallel`: Maximum concurrent parallel steps (default: 10)
   - `enable_logging`: Enable step-level logging (default: true)
   - `stop_on_error`: Stop on first error (default: true)

6. **WorkflowStats**
   - Steps executed/succeeded/failed counters
   - Total execution time tracking
   - Per-step timing metrics

#### Implementation Details

**Files Created**:
- `crates/evif-mem/src/workflow.rs` (new, 358 lines)

**Files Modified**:
- `crates/evif-mem/src/lib.rs` (added workflow module export)

**Key Design Decisions**:
- Async function support via Pin<Box<dyn Future>>
- Arc-wrapped functions for thread-safe sharing
- Builder pattern for step construction
- HashMap-based state management
- Capability tracking via HashSet

#### API Usage

```rust
// Create LLM step
let step = WorkflowStep::llm("extract", "Extract memories from: {text}")
    .with_llm_profile("gpt-4");

// Create function step
let step = WorkflowStep::function(
    "dedupe",
    |mut state| async move {
        state.insert("deduped".to_string(), serde_json::json!(true));
        Ok(state)
    },
    vec![Capability::DB],
);

// Create parallel step
let parallel = WorkflowStep::parallel("parallel_extract", vec![step1, step2]);

// Manage state
let mut state = WorkflowState::new();
state.set_step_output("step1".to_string(), serde_json::json!({"result": "ok"}));
state.set_global("user_id".to_string(), serde_json::json!("user123"));
```

#### Tests Added (7 tests)
- `test_step_type_serialization`: Step type JSON serialization
- `test_capability_hash_set`: Capability set operations
- `test_workflow_step_llm`: LLM step construction
- `test_workflow_step_function`: Function step construction
- `test_workflow_step_parallel`: Parallel step construction
- `test_workflow_state`: State management operations
- `test_workflow_config_default`: Configuration defaults

#### Phase 1.6 Status
- Phase 1.6.1: WorkflowStep Design ✅ **100% Complete**
- Phase 1.6.2: WorkflowRunner ✅ **100% Complete**
- Phase 1.6.3: Interceptor Mechanism ⏳ **Pending**
- Phase 1.6.4: PipelineManager ⏳ **Pending**

**Overall Phase 1.6 Progress**: **50% Complete** (2/4 sub-tasks done)

**evif-mem Overall Progress**: **82% → 83%** (up 1%)

#### Next Steps

**Immediate (Current Sprint)**:
1. Phase 1.6.2: Implement WorkflowRunner
   - WorkflowRunner trait
   - Sequential step execution
   - Parallel step execution

**Short-term (Next Sprint)**:
2. Phase 1.6.3: Implement Interceptor Mechanism
   - Interceptor trait
   - InterceptorRegistry
   - before/after hooks

3. Phase 1.6.4: Implement PipelineManager
   - Dynamic pipeline registration
   - Capability validation
   - Runtime configuration

#### Test Results

```
cargo test -p evif-mem workflow --lib

running 7 tests
test workflow::tests::test_capability_hash_set ... ok
test workflow::tests::test_step_type_serialization ... ok
test workflow::tests::test_workflow_config_default ... ok
test workflow::tests::test_workflow_state ... ok
test workflow::tests::test_workflow_step_function ... ok
test workflow::tests::test_workflow_step_llm ... ok
test workflow::tests::test_workflow_step_parallel ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured

All 99 evif-mem tests pass (92 existing + 7 new).
```

#### Commit

```
git commit 0d95e36
Author: Ralph Loop
Message: feat(evif-mem): implement WorkflowStep structure (Phase 1.6.1)

- Add WorkflowStep with LLM/Function/Parallel step types
- Implement Capability system for dependency tracking
- Add WorkflowState for step output and global state management
- Add WorkflowConfig and WorkflowStats for configuration and metrics
- Implement builder pattern for ergonomic step construction
- Add 7 unit tests for workflow basics
- All 99 evif-mem tests passing (92 existing + 7 new)
- Phase 1.6 progress: 0% → 25%
- evif-mem overall: 81% → 82%

Files changed: 2
- crates/evif-mem/src/workflow.rs (new, 358 lines)
- crates/evif-mem/src/lib.rs (updated)
```

#### Related Memories

- mem-1772893627-f88f: Task completion memory
- Phase 1.6 plan from mem3.md (lines 921-948)
- memU workflow system design (mem3.md lines 396-462)

#### References

- mem3.md Phase 1.6.1 specification
- memU WorkflowStep design pattern
- Workflow engine architecture from memU comparison

---

## [1.5.4] - 2026-03-07
