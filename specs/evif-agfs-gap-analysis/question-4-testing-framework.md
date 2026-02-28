# Question 4: Testing Framework Selection (2025-02-09)

## Context

The pending task `task-1770570814-1998` specifies "Perform systematic E2E testing with Playwright MCP", but the acceptance criteria defined in Question 2 focus on:
- **REST API smoke tests** (30 endpoints)
- **CLI workflow tests** (3 scenarios)

## The Question

**Should we use Playwright MCP for E2E testing, or would a different testing approach be more appropriate for validating REST API and CLI functionality?**

## Why This Matters

- Playwright is primarily designed for **browser automation** and **UI testing**
- The acceptance criteria are **API and CLI focused**, not browser-based
- Using the wrong tool could:
  - Increase complexity (browser overhead for API/CLI tests)
  - Reduce test reliability (unnecessary dependencies)
  - Miss the actual goal (validating production readiness)

## Alternative Perspectives

1. **Playwright MCP** (as specified in task):
   - Pros: Can test REST API via HTTP requests, can test Web UI if needed
   - Cons: Overkill for CLI testing, adds browser dependency, slower execution

2. **Native Rust Tests** (integration tests in `tests/`):
   - Pros: Direct REST API testing via `reqwest`, native CLI testing via `std::process::Command`, faster, more reliable
   - Cons: Requires test environment setup, no UI validation

3. **Shell Script + Curl** (Bash-based E2E):
   - Pros: Lightweight, closest to actual user workflow, easy to read
   - Cons: Less structured, harder to maintain, no assertions framework

4. **Hybrid Approach** (Rust for API, Shell for CLI):
   - Pros: Right tool for each job, clear separation
   - Cons: Two test frameworks to maintain

## Specific Concerns

- Playwright MCP is excellent for **Web UI E2E testing** (validating the 47+ React components)
- But the defined acceptance criteria are **REST + CLI focused**
- Should we expand the acceptance criteria to include Web UI E2E testing, or change the testing framework?
