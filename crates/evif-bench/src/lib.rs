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
