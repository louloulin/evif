// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Traffic Monitoring Module
//!
//! Provides traffic rate calculation (QPS, RPS), bandwidth monitoring,
//! and sliding window statistics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Sliding window bucket
struct WindowBucket {
    /// Timestamp when this bucket was created (Unix timestamp in seconds)
    timestamp: u64,
    /// Request count in this bucket
    count: u64,
    /// Total bytes transferred in this bucket
    bytes: u64,
}

impl WindowBucket {
    fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            count: 0,
            bytes: 0,
        }
    }
}

/// Traffic Monitor
///
/// Provides real-time traffic monitoring with QPS, RPS, and bandwidth calculations
/// using a sliding window algorithm.
pub struct TrafficMonitor {
    /// Total request count
    total_requests: AtomicU64,
    /// Total bytes transferred
    total_bytes: AtomicU64,
    /// Sliding window duration in seconds
    window_secs: u64,
    /// Number of buckets in the sliding window
    num_buckets: usize,
    /// Bucket duration in seconds
    bucket_duration_secs: u64,
    /// Sliding window buckets (protected by mutex for interior mutability)
    buckets: parking_lot::Mutex<Vec<WindowBucket>>,
    /// Last update timestamp
    last_update: parking_lot::Mutex<Instant>,
    /// Current bucket index
    current_bucket: AtomicU64,
}

impl TrafficMonitor {
    /// Create a new TrafficMonitor with specified window duration and bucket count
    ///
    /// # Arguments
    /// * `window_secs` - Total sliding window duration in seconds
    /// * `num_buckets` - Number of buckets (more buckets = higher resolution)
    pub fn new(window_secs: u64, num_buckets: usize) -> Self {
        assert!(window_secs > 0, "window_secs must be > 0");
        assert!(num_buckets > 0, "num_buckets must be > 0");
        assert!(
            window_secs >= num_buckets as u64,
            "window_secs must be >= num_buckets"
        );

        let bucket_duration_secs = window_secs / num_buckets as u64;
        let now = Instant::now();
        let current_time = now.elapsed().as_secs();

        let buckets: Vec<WindowBucket> = (0..num_buckets)
            .map(|i| {
                let offset = ((num_buckets - i) as u64).saturating_mul(bucket_duration_secs);
                WindowBucket::new(current_time.saturating_sub(offset))
            })
            .collect();

        Self {
            total_requests: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            window_secs,
            num_buckets,
            bucket_duration_secs,
            buckets: parking_lot::Mutex::new(buckets),
            last_update: parking_lot::Mutex::new(now),
            current_bucket: AtomicU64::new(0),
        }
    }

    /// Create a TrafficMonitor with default settings (60 second window, 60 buckets)
    pub fn default_monitor() -> Self {
        Self::new(60, 60)
    }

    /// Record a request with the given byte count
    pub fn record_request(&self, bytes: u64) {
        self.record_request_with_time(bytes, Instant::now());
    }

    /// Record a request with explicit timestamp (for testing)
    fn record_request_with_time(&self, bytes: u64, now: Instant) {
        let elapsed = now.elapsed().as_secs();
        let bucket_idx = self.get_bucket_index(elapsed);
        let current_idx = self.current_bucket.load(Ordering::Relaxed);

        // Advance buckets if time has moved forward
        if bucket_idx != current_idx {
            self.advance_buckets(bucket_idx, elapsed);
        }

        // Record in current bucket
        let mut buckets = self.buckets.lock();
        let idx = (bucket_idx % self.num_buckets as u64) as usize;
        buckets[idx].count += 1;
        buckets[idx].bytes += bytes;

        // Update totals
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);

        // Update last update time
        *self.last_update.lock() = now;
    }

    /// Get the bucket index for a given timestamp
    fn get_bucket_index(&self, timestamp: u64) -> u64 {
        timestamp / self.bucket_duration_secs
    }

    /// Advance buckets to the current time
    fn advance_buckets(&self, new_bucket_idx: u64, current_time: u64) {
        let mut buckets = self.buckets.lock();

        // Clear all buckets and reset to current time
        for i in 0..self.num_buckets {
            buckets[i] = WindowBucket::new(current_time);
        }

        self.current_bucket.store(new_bucket_idx, Ordering::Relaxed);
    }

    /// Calculate queries per second (QPS) over the sliding window
    pub fn qps(&self) -> f64 {
        self.requests_in_window() as f64 / self.window_secs as f64
    }

    /// Calculate requests per second (RPS) - alias for qps()
    pub fn rps(&self) -> f64 {
        self.qps()
    }

    /// Calculate bandwidth in bytes per second over the sliding window
    pub fn bandwidth(&self) -> f64 {
        self.bytes_in_window() as f64 / self.window_secs as f64
    }

    /// Calculate bandwidth in human-readable format (KB/s, MB/s, etc.)
    pub fn bandwidth_human(&self) -> String {
        let bps = self.bandwidth();
        if bps >= 1_073_741_824.0 {
            format!("{:.2} GB/s", bps / 1_073_741_824.0)
        } else if bps >= 1_048_576.0 {
            format!("{:.2} MB/s", bps / 1_048_576.0)
        } else if bps >= 1024.0 {
            format!("{:.2} KB/s", bps / 1024.0)
        } else {
            format!("{:.2} B/s", bps)
        }
    }

    /// Get the number of requests in the sliding window
    pub fn requests_in_window(&self) -> u64 {
        let elapsed = self.last_update.lock().elapsed().as_secs();
        let current_time = Instant::now().elapsed().as_secs();
        let current_bucket_idx = self.get_bucket_index(current_time);
        let stored_bucket_idx = self.current_bucket.load(Ordering::Relaxed);

        // If time has advanced, return 0 for the advanced portion
        if current_bucket_idx > stored_bucket_idx + self.num_buckets as u64 {
            return 0;
        }

        let buckets = self.buckets.lock();
        let window_start = current_time.saturating_sub(self.window_secs);

        buckets
            .iter()
            .filter(|b| b.timestamp >= window_start)
            .map(|b| b.count)
            .sum()
    }

    /// Get the number of bytes transferred in the sliding window
    pub fn bytes_in_window(&self) -> u64 {
        let current_time = Instant::now().elapsed().as_secs();
        let window_start = current_time.saturating_sub(self.window_secs);

        let buckets = self.buckets.lock();
        buckets
            .iter()
            .filter(|b| b.timestamp >= window_start)
            .map(|b| b.bytes)
            .sum()
    }

    /// Get total requests since monitor creation
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get total bytes transferred since monitor creation
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    /// Reset all statistics
    pub fn reset(&self) {
        let current_time = Instant::now().elapsed().as_secs();

        let mut buckets = self.buckets.lock();
        for i in 0..self.num_buckets {
            buckets[i] = WindowBucket::new(current_time);
        }

        self.total_requests.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        *self.last_update.lock() = Instant::now();
        self.current_bucket.store(self.get_bucket_index(current_time), Ordering::Relaxed);
    }

    /// Get current statistics snapshot
    pub fn stats(&self) -> TrafficStats {
        TrafficStats {
            qps: self.qps(),
            rps: self.rps(),
            bandwidth: self.bandwidth(),
            bandwidth_human: self.bandwidth_human(),
            requests_in_window: self.requests_in_window(),
            bytes_in_window: self.bytes_in_window(),
            total_requests: self.total_requests(),
            total_bytes: self.total_bytes(),
        }
    }
}

/// Traffic statistics snapshot
#[derive(Debug, Clone)]
pub struct TrafficStats {
    /// Queries per second
    pub qps: f64,
    /// Requests per second (same as QPS)
    pub rps: f64,
    /// Bandwidth in bytes per second
    pub bandwidth: f64,
    /// Bandwidth in human-readable format
    pub bandwidth_human: String,
    /// Requests in current window
    pub requests_in_window: u64,
    /// Bytes in current window
    pub bytes_in_window: u64,
    /// Total requests since creation
    pub total_requests: u64,
    /// Total bytes since creation
    pub total_bytes: u64,
}

impl Default for TrafficMonitor {
    fn default() -> Self {
        Self::default_monitor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_monitor_creation() {
        let monitor = TrafficMonitor::new(60, 60);
        assert_eq!(monitor.window_secs, 60);
        assert_eq!(monitor.num_buckets, 60);
    }

    #[test]
    fn test_record_request() {
        let monitor = TrafficMonitor::new(10, 10);

        // Record some requests
        monitor.record_request(100);
        monitor.record_request(200);
        monitor.record_request(300);

        assert_eq!(monitor.total_requests(), 3);
        assert_eq!(monitor.total_bytes(), 600);
    }

    #[test]
    fn test_qps_calculation() {
        let monitor = TrafficMonitor::new(10, 10);

        // Record 10 requests
        for _ in 0..10 {
            monitor.record_request(100);
        }

        // QPS should be close to 1 (10 requests / 10 seconds)
        let qps = monitor.qps();
        assert!(qps > 0.0, "QPS should be > 0");
        assert!(qps <= 2.0, "QPS should be <= 2 for fresh monitor");
    }

    #[test]
    fn test_bandwidth_calculation() {
        let monitor = TrafficMonitor::new(10, 10);

        // Record requests with known bytes
        monitor.record_request(1000);
        monitor.record_request(1000);

        let bandwidth = monitor.bandwidth();
        assert!(bandwidth >= 0.0, "Bandwidth should be >= 0");

        let human = monitor.bandwidth_human();
        assert!(human.contains("B/s") || human.contains("KB/s"), "Should be in human-readable format");
    }

    #[test]
    fn test_reset() {
        let monitor = TrafficMonitor::new(10, 10);

        monitor.record_request(100);
        assert_eq!(monitor.total_requests(), 1);

        monitor.reset();
        assert_eq!(monitor.total_requests(), 0);
        assert_eq!(monitor.total_bytes(), 0);
    }

    #[test]
    fn test_stats_snapshot() {
        let monitor = TrafficMonitor::new(10, 10);
        monitor.record_request(500);

        let stats = monitor.stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_bytes, 500);
        assert!(stats.qps >= 0.0);
        assert!(stats.bandwidth >= 0.0);
    }

    #[test]
    fn test_bandwidth_human_formats() {
        let monitor = TrafficMonitor::new(10, 10);

        // Test with small bandwidth
        monitor.record_request(100);
        let human = monitor.bandwidth_human();
        assert!(human.contains("B/s") || human.contains("KB/s"));

        // Test with large bandwidth (simulate by manually checking format)
        let large_monitor = TrafficMonitor::new(10, 10);
        for _ in 0..1000 {
            large_monitor.record_request(1024 * 1024); // 1 MB each
        }
        let large_human = large_monitor.bandwidth_human();
        assert!(
            large_human.contains("KB/s") || large_human.contains("MB/s") || large_human.contains("GB/s"),
            "Should be in KB/s, MB/s or GB/s format"
        );
    }

    #[test]
    fn test_default_monitor() {
        let monitor = TrafficMonitor::default_monitor();
        assert_eq!(monitor.window_secs, 60);
        assert_eq!(monitor.num_buckets, 60);
    }

    #[test]
    #[should_panic]
    fn test_invalid_window_secs() {
        TrafficMonitor::new(0, 10);
    }

    #[test]
    #[should_panic]
    fn test_invalid_num_buckets() {
        TrafficMonitor::new(60, 0);
    }
}
