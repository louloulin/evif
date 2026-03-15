// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::backend::Transaction;
use crate::{StorageError, StorageResult};
use async_trait::async_trait;

/// 内存事务
pub struct MemoryTransaction {
    operations: Vec<crate::StorageOp>,
    committed: bool,
}

impl MemoryTransaction {
    pub fn new() -> Self {
        MemoryTransaction {
            operations: Vec::new(),
            committed: false,
        }
    }

    pub fn add_operation(&mut self, op: crate::StorageOp) {
        self.operations.push(op);
    }

    pub fn operations(&self) -> &[crate::StorageOp] {
        &self.operations
    }
}

impl Default for MemoryTransaction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::backend::Transaction for MemoryTransaction {
    async fn commit(mut self: Box<Self>) -> StorageResult<()> {
        self.committed = true;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> StorageResult<()> {
        self.operations.clear();
        Ok(())
    }
}

/// 事务管理器
pub struct TransactionManager {
    // 在真实实现中，这里会管理事务状态
}

impl TransactionManager {
    pub fn new() -> Self {
        TransactionManager {}
    }

    pub fn create_transaction(&self) -> MemoryTransaction {
        MemoryTransaction::new()
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = MemoryTransaction::new();
        assert!(!tx.committed);
        assert_eq!(tx.operations().len(), 0);
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let tx = MemoryTransaction::new();
        let boxed = Box::new(tx);
        let result = boxed.commit().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_transaction_manager() {
        let manager = TransactionManager::new();
        let tx = manager.create_transaction();
        assert_eq!(tx.operations().len(), 0);
    }
}
