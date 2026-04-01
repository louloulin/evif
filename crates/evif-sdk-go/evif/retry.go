package evif

import (
	"context"
	"fmt"
	"io"
	"math"
	"math/rand"
	"net/http"
	"net/url"
	"sync"
	"time"
)

// RetryConfig configures retry behavior.
type RetryConfig struct {
	MaxRetries     int           // Maximum number of retries (default: 3)
	InitialBackoff time.Duration // Initial backoff duration (default: 100ms)
	MaxBackoff     time.Duration // Maximum backoff duration (default: 30s)
	Jitter         bool          // Add random jitter to backoff (default: true)
}

// DefaultRetryConfig returns the default retry configuration.
func DefaultRetryConfig() *RetryConfig {
	return &RetryConfig{
		MaxRetries:     3,
		InitialBackoff: 100 * time.Millisecond,
		MaxBackoff:     30 * time.Second,
		Jitter:         true,
	}
}

// CircuitBreakerState represents the state of a circuit breaker.
type CircuitBreakerState int

const (
	CircuitClosed CircuitBreakerState = iota
	CircuitOpen
	CircuitHalfOpen
)

func (s CircuitBreakerState) String() string {
	switch s {
	case CircuitClosed:
		return "closed"
	case CircuitOpen:
		return "open"
	case CircuitHalfOpen:
		return "half-open"
	default:
		return "unknown"
	}
}

// CircuitBreakerConfig configures circuit breaker behavior.
type CircuitBreakerConfig struct {
	FailureThreshold int           // Failures before opening circuit (default: 5)
	SuccessThreshold int           // Successes to close circuit in half-open (default: 2)
	Timeout          time.Duration // Time before attempting recovery (default: 30s)
}

// DefaultCircuitBreakerConfig returns the default circuit breaker configuration.
func DefaultCircuitBreakerConfig() *CircuitBreakerConfig {
	return &CircuitBreakerConfig{
		FailureThreshold: 5,
		SuccessThreshold: 2,
		Timeout:          30 * time.Second,
	}
}

// CircuitBreaker implements the circuit breaker pattern.
type CircuitBreaker struct {
	mu               sync.Mutex
	state            CircuitBreakerState
	failures         int
	successes        int
	lastFailureTime  time.Time
	config           *CircuitBreakerConfig
}

// NewCircuitBreaker creates a new circuit breaker with the given config.
func NewCircuitBreaker(config *CircuitBreakerConfig) *CircuitBreaker {
	if config == nil {
		config = DefaultCircuitBreakerConfig()
	}
	return &CircuitBreaker{
		state:  CircuitClosed,
		config: config,
	}
}

// Allow checks if a request should be allowed.
func (cb *CircuitBreaker) Allow() bool {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	switch cb.state {
	case CircuitClosed:
		return true
	case CircuitOpen:
		if time.Since(cb.lastFailureTime) > cb.config.Timeout {
			cb.state = CircuitHalfOpen
			cb.successes = 0
			return true
		}
		return false
	case CircuitHalfOpen:
		return true
	default:
		return false
	}
}

// RecordSuccess records a successful request.
func (cb *CircuitBreaker) RecordSuccess() {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	switch cb.state {
	case CircuitHalfOpen:
		cb.successes++
		if cb.successes >= cb.config.SuccessThreshold {
			cb.state = CircuitClosed
			cb.failures = 0
		}
	case CircuitClosed:
		cb.failures = 0
	}
}

// RecordFailure records a failed request.
func (cb *CircuitBreaker) RecordFailure() {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	cb.lastFailureTime = time.Now()

	switch cb.state {
	case CircuitHalfOpen:
		cb.state = CircuitOpen
	case CircuitClosed:
		cb.failures++
		if cb.failures >= cb.config.FailureThreshold {
			cb.state = CircuitOpen
		}
	}
}

// State returns the current state of the circuit breaker.
func (cb *CircuitBreaker) State() CircuitBreakerState {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	return cb.state
}

// calculateBackoff calculates the next backoff duration.
func calculateBackoff(attempt int, config *RetryConfig) time.Duration {
	backoff := float64(config.InitialBackoff) * math.Pow(2, float64(attempt))
	if backoff > float64(config.MaxBackoff) {
		backoff = float64(config.MaxBackoff)
	}

	if config.Jitter {
		// Add random jitter: +/- 25%
		jitter := backoff * 0.25 * (2*rand.Float64() - 1)
		backoff += jitter
	}

	return time.Duration(backoff)
}

// isRetryableStatus checks if an HTTP status code should trigger a retry.
func isRetryableStatus(statusCode int) bool {
	// Retry on 5xx errors and 429 (Too Many Requests)
	return statusCode >= 500 || statusCode == 429
}

// isRetryableError checks if an error should trigger a retry.
func isRetryableError(err error) bool {
	if err == nil {
		return false
	}
	// Retry on temporary/network errors (url.Error wraps network errors)
	if _, ok := err.(*url.Error); ok {
		return true
	}
	// Retry on EOF and temporary I/O errors
	if err == io.EOF {
		return true
	}
	return false
}

// RetryOptions holds optional retry parameters.
type RetryOptions struct {
	Config        *RetryConfig        // Retry configuration
	CircuitBreaker *CircuitBreaker   // Optional circuit breaker
	ShouldRetry   func(int, *http.Response, error) bool // Custom retry predicate
}

// RetryOption is a function that modifies RetryOptions.
type RetryOption func(*RetryOptions)

// WithRetryConfig sets the retry configuration.
func WithRetryConfig(config *RetryConfig) RetryOption {
	return func(o *RetryOptions) {
		o.Config = config
	}
}

// WithCircuitBreaker sets a circuit breaker.
func WithCircuitBreaker(cb *CircuitBreaker) RetryOption {
	return func(o *RetryOptions) {
		o.CircuitBreaker = cb
	}
}

// WithShouldRetry sets a custom retry predicate.
func WithShouldRetry(shouldRetry func(int, *http.Response, error) bool) RetryOption {
	return func(o *RetryOptions) {
		o.ShouldRetry = shouldRetry
	}
}

// ExecuteWithRetry executes fn with retry logic, circuit breaker, and proper backoff.
// The fn should perform the HTTP request and return the response and error.
func ExecuteWithRetry(
	ctx context.Context,
	fn func() (*http.Response, error),
	options ...RetryOption,
) (*http.Response, error) {
	opts := &RetryOptions{
		Config: DefaultRetryConfig(),
	}
	for _, o := range options {
		o(opts)
	}

	var lastResp *http.Response
	var lastErr error

	for attempt := 0; attempt <= opts.Config.MaxRetries; attempt++ {
		// Check circuit breaker
		if opts.CircuitBreaker != nil && !opts.CircuitBreaker.Allow() {
			return nil, fmt.Errorf("circuit breaker is open")
		}

		// Execute the request
		resp, err := fn()
		lastResp = resp
		lastErr = err

		// Record circuit breaker result
		if opts.CircuitBreaker != nil {
			if err != nil || (resp != nil && resp.StatusCode >= 500) {
				opts.CircuitBreaker.RecordFailure()
			} else {
				opts.CircuitBreaker.RecordSuccess()
			}
		}

		// Check if we should retry
		if attempt == opts.Config.MaxRetries {
			break
		}

		shouldRetry := false
		if opts.ShouldRetry != nil {
			shouldRetry = opts.ShouldRetry(attempt, resp, err)
		} else {
			shouldRetry = isRetryableError(err) || (resp != nil && isRetryableStatus(resp.StatusCode))
		}

		if !shouldRetry {
			break
		}

		// Close response body if present
		if resp != nil {
			resp.Body.Close()
		}

		// Calculate and apply backoff
		backoff := calculateBackoff(attempt, opts.Config)
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-time.After(backoff):
		}
	}

	return lastResp, lastErr
}

// ClientOption is a function that modifies Client.
type ClientOption func(*Client)

// WithRetryConfig applies a retry configuration to the client.
func WithRetryConfigClient(config *RetryConfig) ClientOption {
	return func(c *Client) {
		c.retryConfig = config
	}
}

// WithCircuitBreaker applies a circuit breaker to the client.
func WithCircuitBreakerClient(cb *CircuitBreaker) ClientOption {
	return func(c *Client) {
		c.circuitBreaker = cb
	}
}
