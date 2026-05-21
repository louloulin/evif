// Phase 14.4: evif-bench - Performance Benchmark Suite
//
// OSWorld, IDE-Bench, AgentBench, L0CO benchmarks

pub mod agentbench;
pub mod idebench;
pub mod l0co;
pub mod osworld;
pub mod performance;

use evif_core::RadixMountTable;
use std::sync::Arc;

/// 创建测试用的 RadixMountTable
pub fn test_mount_table() -> Arc<RadixMountTable> {
    Arc::new(RadixMountTable::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_mount_table_creates_radix_mount_table() {
        let table = test_mount_table();
        // Basic smoke test - table is created
        assert!(Arc::strong_count(&table) >= 1);
    }

    #[test]
    fn test_agentbench_module_exists() {
        // Verify module is properly declared
        let benches = agentbench::benchmarks();
        assert_eq!(benches.len(), 6);
    }

    #[test]
    fn test_idebench_module_exists() {
        // Verify module is properly declared
        let benches = idebench::benchmarks();
        assert_eq!(benches.len(), 5);
    }

    #[test]
    fn test_l0co_module_exists() {
        // Verify module is properly declared
        let benches = l0co::benchmarks();
        assert_eq!(benches.len(), 3);
    }

    #[test]
    fn test_osworld_module_exists() {
        // Verify module is properly declared
        let benches = osworld::benchmarks();
        assert_eq!(benches.len(), 3);
    }

    #[test]
    fn test_performance_module_exists() {
        // Verify module is properly declared
        let benches = performance::benchmarks();
        assert_eq!(benches.len(), 4);
    }
}
