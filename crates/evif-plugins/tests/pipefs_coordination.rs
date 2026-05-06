// PipeFS Multi-Agent Coordination Tests
//
// Test wait_for_result, try_claim, and state machine enforcement.

use std::sync::Arc;
use std::time::Duration;

use evif_core::{EvifError, EvifPlugin, WriteFlags};
use evif_plugins::PipeFsPlugin;

/// Test 1: try_claim succeeds when pipe has no assignee.
#[tokio::test]
async fn test_try_claim_succeeds_when_unclaimed() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/claim-test", 0o755).await.expect("mkdir");

    let result = plugin.try_claim("claim-test", "agent-1").await;
    assert!(result.is_ok(), "First claim should succeed");

    let assignee = plugin.read("/claim-test/assignee", 0, 0).await.expect("read assignee");
    assert_eq!(assignee, b"agent-1");
}

/// Test 2: try_claim fails when already claimed by another agent.
#[tokio::test]
async fn test_try_claim_fails_when_already_claimed() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/claim-test-2", 0o755).await.expect("mkdir");

    // First agent claims
    plugin.try_claim("claim-test-2", "agent-1").await.expect("first claim");

    // Second agent fails to claim
    let err = plugin.try_claim("claim-test-2", "agent-2").await.expect_err("second claim should fail");
    match err {
        EvifError::InvalidInput(msg) => {
            assert!(msg.contains("already claimed"), "Should report already claimed: {}", msg);
        }
        _ => panic!("Expected InvalidInput error, got {:?}", err),
    }
}

/// Test 3: try_claim succeeds when same agent re-claims.
#[tokio::test]
async fn test_try_claim_same_agent_succeeds() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/claim-same", 0o755).await.expect("mkdir");

    plugin.try_claim("claim-same", "agent-1").await.expect("first claim");
    let result = plugin.try_claim("claim-same", "agent-1").await;
    assert!(result.is_ok(), "Same agent re-claim should succeed");
}

/// Test 4: wait_for_result returns output when completed.
#[tokio::test]
async fn test_wait_for_result_returns_output() {
    let plugin = Arc::new(PipeFsPlugin::new());
    plugin.mkdir("/wait-test", 0o755).await.expect("mkdir");

    // Spawn a task that writes output after a short delay
    let plugin_clone = Arc::clone(&plugin);
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        plugin_clone
            .write("/wait-test/output", b"result data".to_vec(), 0, WriteFlags::TRUNCATE)
            .await
            .expect("write output");
    });

    // Wait for result
    let result = plugin.wait_for_result("wait-test", Duration::from_secs(5)).await;
    assert!(result.is_ok(), "wait should succeed");

    let output = result.unwrap();
    assert_eq!(output, b"result data");

    handle.await.expect("task join");
}

/// Test 5: wait_for_result times out when no output.
#[tokio::test]
async fn test_wait_for_result_times_out() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/wait-timeout", 0o755).await.expect("mkdir");

    // Set timeout to 0 to make it expire immediately on wait
    plugin
        .write("/wait-timeout/timeout", b"0".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("set timeout");

    let err = plugin
        .wait_for_result("wait-timeout", Duration::from_millis(50))
        .await
        .expect_err("should timeout");
    match err {
        EvifError::InvalidInput(msg) => {
            assert!(msg.contains("timed out"), "Should report timeout: {}", msg);
        }
        _ => panic!("Expected InvalidInput error for timeout, got {:?}", err),
    }
}

/// Test 6: state transition from pending to running is valid.
#[tokio::test]
async fn test_state_pending_to_running() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/state-1", 0o755).await.expect("mkdir");

    // Writing input transitions from pending to running
    plugin
        .write("/state-1/input", b"task".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write input");

    let status = plugin.read("/state-1/status", 0, 0).await.expect("read status");
    assert_eq!(status, b"running");
}

/// Test 7: state transition from running to completed is valid.
#[tokio::test]
async fn test_state_running_to_completed() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/state-2", 0o755).await.expect("mkdir");

    // Set to running first
    plugin
        .write("/state-2/input", b"task".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write input");

    // Writing output transitions to completed
    plugin
        .write("/state-2/output", b"done".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write output");

    let status = plugin.read("/state-2/status", 0, 0).await.expect("read status");
    assert_eq!(status, b"completed");
}

/// Test 8: invalid state transition is rejected.
#[tokio::test]
async fn test_invalid_state_transition_rejected() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/state-3", 0o755).await.expect("mkdir");

    // Pending -> skip directly to completed (invalid)
    let err = plugin
        .write("/state-3/status", b"completed".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect_err("should reject invalid transition");
    match err {
        EvifError::InvalidInput(msg) => {
            assert!(msg.contains("Invalid state transition"), "Should report invalid transition: {}", msg);
        }
        _ => panic!("Expected InvalidInput error, got {:?}", err),
    }
}

/// Test 9: error state can transition back to pending (retry).
#[tokio::test]
async fn test_state_error_to_pending_retry() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/state-4", 0o755).await.expect("mkdir");

    // Set to error
    plugin
        .write("/state-4/status", b"error".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("set error");

    // Reset back to pending for retry
    let result = plugin
        .write("/state-4/status", b"pending".to_vec(), 0, WriteFlags::TRUNCATE)
        .await;
    assert!(result.is_ok(), "error -> pending transition should be allowed (retry)");
}

/// Test 10: timeout state can transition back to pending (retry).
#[tokio::test]
async fn test_state_timeout_to_pending_retry() {
    let plugin = PipeFsPlugin::new();
    plugin.mkdir("/state-5", 0o755).await.expect("mkdir");

    // Set to timeout
    plugin
        .write("/state-5/status", b"timeout".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("set timeout");

    // Reset back to pending for retry
    let result = plugin
        .write("/state-5/status", b"pending".to_vec(), 0, WriteFlags::TRUNCATE)
        .await;
    assert!(result.is_ok(), "timeout -> pending transition should be allowed (retry)");
}
