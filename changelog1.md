# Changelog

All notable changes to the evif-mem project will be documented in this file.

## [Unreleased]

### Added - Phase 1.6: Workflow System (In Progress)

**WorkflowRunner Implementation** - Workflow execution engine with sequential and parallel support

#### Phase 1.6.2: WorkflowRunner

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
