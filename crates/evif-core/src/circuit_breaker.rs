// Circuit Breaker Pattern — prevents cascade failures from downstream services
//
// Three states:
//   Closed  → Normal operation; requests pass through.
//              On failure: increment failure counter.
//              If failures >= threshold: transition to Open.
//   Open    → Fast-fail; all requests return CircuitOpen immediately.
//              After recovery_timeout_secs: transition to HalfOpen.
//   HalfOpen → Testing; allow up to half_open_max_calls through.
//              On success: transition to Closed.
//              On failure: transition back to Open.
//
// Usage:
//   let cb = CircuitBreaker::new("llm", CircuitBreakerConfig::default());
//   let result = cb.execute(|| async { call_llm().await }).await;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Three-state circuit breaker
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    /// Current number of consecutive failures
    failure_count: AtomicUsize,
    /// Number of calls in half-open state (to enforce half_open_max_calls)
    half_open_calls: AtomicUsize,
    /// Timestamp (Unix millis) when circuit last opened
    last_failure_ts: AtomicU64,
    /// Timestamp (Unix millis) when circuit was closed (reset)
    last_success_ts: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: usize,
    /// Seconds to wait before transitioning Open → HalfOpen
    pub recovery_timeout_secs: u64,
    /// Max concurrent calls to allow in HalfOpen state
    pub half_open_max_calls: usize,
    /// Minimum number of successful calls in HalfOpen before closing
    pub half_open_success_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_secs: 30,
            half_open_max_calls: 3,
            half_open_success_threshold: 2,
        }
    }
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            failure_count: AtomicUsize::new(0),
            half_open_calls: AtomicUsize::new(0),
            last_failure_ts: AtomicU64::new(0),
            last_success_ts: AtomicU64::new(now_millis()),
        }
    }

    /// Execute an async operation through the circuit breaker.
    /// Returns Err(CircuitBreakerError::Open) if the circuit is open.
    pub async fn execute<F, Fut, T>(&self, op: F) -> Result<T, CircuitBreakerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<
            Output = Result<T, std::sync::Arc<dyn std::error::Error + Send + Sync>>,
        >,
    {
        // Fast path: check if we should attempt the call
        match self.state() {
            CircuitState::Open => {
                return Err(CircuitBreakerError::Open {
                    name: self.name.clone(),
                });
            }
            CircuitState::HalfOpen => {
                // Limit concurrent half-open calls
                let calls = self.half_open_calls.fetch_add(1, Ordering::Acquire);
                if calls >= self.config.half_open_max_calls {
                    self.half_open_calls.fetch_sub(1, Ordering::Release);
                    return Err(CircuitBreakerError::HalfOpenBusy {
                        name: self.name.clone(),
                    });
                }
                // Will decrement in any case below
                let _guard = HalfOpenGuard(&self.half_open_calls);
            }
            CircuitState::Closed => {}
        }

        // Attempt the operation
        let start = Instant::now();
        let result = op().await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(value) => {
                self.on_success(latency_ms);
                Ok(value)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::DownstreamError {
                    name: self.name.clone(),
                    source: e,
                })
            }
        }
    }

    fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Acquire);

        if failures == 0 {
            return CircuitState::Closed;
        }

        let last_failure = self.last_failure_ts.load(Ordering::Acquire);

        let elapsed_secs = now_millis().saturating_sub(last_failure) as f64 / 1000.0;

        if elapsed_secs >= self.config.recovery_timeout_secs as f64 {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    fn on_success(&self, latency_ms: u64) {
        self.failure_count.store(0, Ordering::Release);
        self.last_success_ts.store(now_millis(), Ordering::Release);

        let half_calls = self.half_open_calls.load(Ordering::Acquire);
        if half_calls > 0 {
            // Success in half-open — circuit is recovering
            tracing::debug!(
                name = %self.name,
                half_open_calls = half_calls,
                latency_ms = latency_ms,
                "Circuit breaker: downstream recovered"
            );
        } else {
            tracing::trace!(
                name = %self.name,
                latency_ms = latency_ms,
                "Circuit breaker: request succeeded"
            );
        }
    }

    fn on_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
        self.last_failure_ts.store(now_millis(), Ordering::Release);

        if count >= self.config.failure_threshold {
            tracing::warn!(
                name = %self.name,
                failures = count,
                threshold = self.config.failure_threshold,
                "Circuit breaker OPEN — downstream failures exceeded threshold"
            );
        } else {
            tracing::debug!(
                name = %self.name,
                failures = count,
                threshold = self.config.failure_threshold,
                "Circuit breaker: downstream failure recorded"
            );
        }
    }

    /// Returns a snapshot of the current circuit state for monitoring.
    pub fn snapshot(&self) -> CircuitBreakerSnapshot {
        let state = self.state();
        let failures = self.failure_count.load(Ordering::Relaxed);
        let half_open_calls = self.half_open_calls.load(Ordering::Relaxed);
        let last_failure_ts = self.last_failure_ts.load(Ordering::Relaxed);
        let last_success_ts = self.last_success_ts.load(Ordering::Relaxed);

        CircuitBreakerSnapshot {
            name: self.name.clone(),
            state: state.clone(),
            consecutive_failures: failures,
            half_open_calls,
            last_failure_ts,
            last_success_ts,
        }
    }

    /// Manually reset the circuit to Closed state.
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Release);
        self.half_open_calls.store(0, Ordering::Release);
        self.last_success_ts.store(now_millis(), Ordering::Release);
        tracing::info!(name = %self.name, "Circuit breaker manually reset to CLOSED");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerSnapshot {
    pub name: String,
    pub state: CircuitState,
    pub consecutive_failures: usize,
    pub half_open_calls: usize,
    pub last_failure_ts: u64,
    pub last_success_ts: u64,
}

#[derive(Debug)]
pub enum CircuitBreakerError {
    /// Circuit is open — downstream is unavailable
    Open { name: String },
    /// Half-open slot exhausted — too many concurrent test calls
    HalfOpenBusy { name: String },
    /// Downstream call failed
    DownstreamError {
        name: String,
        source: Arc<dyn std::error::Error + Send + Sync>,
    },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { name } => write!(
                f,
                "CircuitBreaker({}) is OPEN — downstream unavailable",
                name
            ),
            Self::HalfOpenBusy { name } => {
                write!(f, "CircuitBreaker({}) half-open slot exhausted", name)
            }
            Self::DownstreamError { name, .. } => {
                write!(f, "CircuitBreaker({}) downstream error", name)
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DownstreamError { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

struct HalfOpenGuard<'a>(&'a AtomicUsize);

impl Drop for HalfOpenGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Global registry of circuit breakers — one per downstream service.
use std::sync::Mutex;
use std::sync::OnceLock;

static CIRCUIT_BREAKERS: OnceLock<Mutex<std::collections::HashMap<String, Arc<CircuitBreaker>>>> =
    OnceLock::new();

fn get_registry() -> &'static Mutex<std::collections::HashMap<String, Arc<CircuitBreaker>>> {
    CIRCUIT_BREAKERS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Get or create a named circuit breaker.
pub fn get_circuit_breaker(name: &str) -> Arc<CircuitBreaker> {
    let registry = get_registry();
    let mut guard = registry.lock().unwrap();
    guard
        .entry(name.to_string())
        .or_insert_with(|| Arc::new(CircuitBreaker::new(name, CircuitBreakerConfig::default())))
        .clone()
}

/// Returns snapshots of all registered circuit breakers.
pub fn all_circuit_breakers() -> Vec<CircuitBreakerSnapshot> {
    let registry = get_registry();
    let guard = registry.lock().unwrap();
    guard.values().map(|cb| cb.snapshot()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError(&'static str);
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::error::Error for TestError {}
    unsafe impl Send for TestError {}
    unsafe impl Sync for TestError {}

    #[tokio::test]
    async fn circuit_stays_closed_on_success() {
        let cb = CircuitBreaker::new("test-ok", CircuitBreakerConfig::default());

        for _ in 0..5 {
            let r = cb
                .execute(|| async { Ok::<_, Arc<dyn std::error::Error + Send + Sync>>(42) })
                .await;
            assert!(r.is_ok());
            assert_eq!(r.unwrap(), 42);
        }

        assert_eq!(cb.snapshot().state, CircuitState::Closed);
        assert_eq!(cb.snapshot().consecutive_failures, 0);
    }

    #[tokio::test]
    async fn circuit_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-open", config);

        for _ in 0..3 {
            // Clone inside loop so closure stays Fn (not FnOnce)
            let err: Arc<dyn std::error::Error + Send + Sync> = Arc::new(TestError("down"));
            cb.execute(move || {
                let e = err.clone();
                async move { Err::<(), _>(e) }
            })
            .await
            .unwrap_err();
        }

        // Now circuit should be open — fast-fail
        let r = cb
            .execute(|| async { Ok::<_, Arc<dyn std::error::Error + Send + Sync>>(99) })
            .await;
        assert!(matches!(r, Err(CircuitBreakerError::Open { .. })));
    }

    #[tokio::test]
    async fn circuit_reset_closes_it() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test-reset", config);

        for _ in 0..2 {
            let err: Arc<dyn std::error::Error + Send + Sync> = Arc::new(TestError("down"));
            cb.execute(move || {
                let e = err.clone();
                async move { Err::<(), _>(e) }
            })
            .await
            .unwrap_err();
        }

        cb.reset();
        let r = cb
            .execute(|| async { Ok::<_, Arc<dyn std::error::Error + Send + Sync>>(1) })
            .await;
        assert!(r.is_ok());
    }
}
