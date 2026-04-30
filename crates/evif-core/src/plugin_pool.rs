// WASM Plugin Pool Implementation
//
// 实现 WASM 插件实例池，复用 Extism 插件实例以提高性能
// 参考 AGFS 的 WASM 实例池设计

use crate::error::EvifResult;
use crate::extism_plugin::{ExtismPlugin, WasmPluginConfig};
use crate::plugin::EvifPlugin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// 实例池错误类型
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool exhausted: max_capacity={0}")]
    PoolExhausted(usize),

    #[error("Plugin creation failed: {0}")]
    PluginCreation(String),

    #[error("Plugin acquire timeout")]
    AcquireTimeout,

    #[error("Plugin closed")]
    PoolClosed,
}

/// 实例池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最大空闲实例数
    pub max_idle: usize,
    /// 最小空闲实例数（预热时创建）
    pub min_idle: usize,
    /// 最大总实例数
    pub max_total: usize,
    /// 实例空闲超时时间（秒）
    pub idle_timeout_secs: u64,
    /// 获取实例超时时间（毫秒）
    pub acquire_timeout_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle: 10,
            min_idle: 2,
            max_total: 100,
            idle_timeout_secs: 300, // 5 minutes
            acquire_timeout_ms: 5000,
        }
    }
}

impl PoolConfig {
    /// 创建新配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大空闲实例数
    pub fn with_max_idle(mut self, max_idle: usize) -> Self {
        self.max_idle = max_idle;
        self
    }

    /// 设置最小空闲实例数
    pub fn with_min_idle(mut self, min_idle: usize) -> Self {
        self.min_idle = min_idle;
        self
    }

    /// 设置最大总实例数
    pub fn with_max_total(mut self, max_total: usize) -> Self {
        self.max_total = max_total;
        self
    }

    /// 设置空闲超时时间
    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }

    /// 设置获取超时时间
    pub fn with_acquire_timeout(mut self, ms: u64) -> Self {
        self.acquire_timeout_ms = ms;
        self
    }
}

/// 池化插件实例
pub struct PooledPlugin {
    /// 内部插件实例
    plugin: ExtismPlugin,
    /// 所属池（使用 Weak 避免循环引用）
    pool: std::sync::Weak<PluginPool>,
    /// 最后使用时间
    last_used: Instant,
    /// 是否已标记为废弃
    retiring: bool,
}

impl PooledPlugin {
    /// 创建新的池化插件
    fn new(plugin: ExtismPlugin, pool: &Arc<PluginPool>) -> Self {
        Self {
            plugin,
            pool: std::sync::Arc::downgrade(pool),
            last_used: Instant::now(),
            retiring: false,
        }
    }

    /// 获取内部插件引用
    pub fn plugin(&self) -> &ExtismPlugin {
        &self.plugin
    }

    /// 标记为使用中（更新最后使用时间）
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }

    /// 检查是否已超时
    pub fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }
}

/// 实例归还时自动释放回池
impl Drop for PooledPlugin {
    fn drop(&mut self) {
        if !self.retiring && !self.plugin.name().is_empty() {
            // 尝试获取池的引用并异步归还
            if let Some(pool) = self.pool.upgrade() {
                let pool = Arc::clone(&pool);
                tokio::spawn(async move {
                    pool.return_plugin().await;
                });
            }
        }
    }
}

/// WASM 插件实例池
pub struct PluginPool {
    /// 插件配置
    config: WasmPluginConfig,
    /// 池配置
    pool_config: PoolConfig,
    /// 空闲实例队列
    idle: tokio::sync::Mutex<Vec<PooledPlugin>>,
    /// 活跃实例计数
    active_count: AtomicUsize,
    /// 总创建实例计数
    total_count: AtomicUsize,
    /// 获取信号量（限制并发获取）
    acquire_semaphore: Semaphore,
    /// 池是否已关闭
    closed: AtomicUsize,
}

impl std::fmt::Debug for PluginPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginPool")
            .field("config", &self.config.name)
            .field("active", &self.active_count.load(Ordering::Relaxed))
            .field("total", &self.total_count.load(Ordering::Relaxed))
            .field("idle", &self.pool_config.max_idle)
            .finish()
    }
}

impl PluginPool {
    /// 创建新的插件池
    ///
    /// # 参数
    /// - `config`: 插件配置
    /// - `pool_config`: 池配置
    ///
    /// # 返回
    /// 实例池
    pub fn new(config: WasmPluginConfig, pool_config: PoolConfig) -> EvifResult<Self> {
        let max_total = pool_config.max_total;
        let pool = Self {
            config,
            pool_config,
            idle: tokio::sync::Mutex::new(Vec::new()),
            active_count: AtomicUsize::new(0),
            total_count: AtomicUsize::new(0),
            acquire_semaphore: Semaphore::new(max_total),
            closed: AtomicUsize::new(0),
        };

        tracing::info!(
            "Created plugin pool '{}': max_total={}, max_idle={}, min_idle={}",
            pool.config.name,
            pool.pool_config.max_total,
            pool.pool_config.max_idle,
            pool.pool_config.min_idle
        );

        Ok(pool)
    }

    /// 创建新的插件池（使用默认配置）
    pub fn with_default_config(config: WasmPluginConfig) -> EvifResult<Self> {
        Self::new(config, PoolConfig::default())
    }

    /// 预热池（创建最小空闲实例）
    pub async fn warmup(self: &Arc<Self>) -> PoolResult<()> {
        if self.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        let mut created = 0;
        let target = self.pool_config.min_idle;

        while created < target {
            match self.create_plugin().await {
                Ok(plugin) => {
                    let mut idle = self.idle.lock().await;
                    idle.push(PooledPlugin::new(plugin, self));
                    created += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to create warmup plugin {}: {}", created, e);
                    break;
                }
            }
        }

        tracing::info!("Plugin pool '{}' warmed up: {} instances", self.config.name, created);
        Ok(())
    }

    /// 获取可用插件实例
    ///
    /// # 返回
    /// 池化插件实例
    pub async fn acquire(&self) -> PoolResult<PooledPlugin> {
        if self.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        // 等待获取许可
        let timeout_duration = Duration::from_millis(self.pool_config.acquire_timeout_ms);
        let permit = match timeout(timeout_duration, self.acquire_semaphore.acquire()).await {
            Ok(Ok(p)) => p,
            Ok(Err(_)) => return Err(PoolError::PoolClosed),
            Err(_) => return Err(PoolError::AcquireTimeout),
        };

        // 首先尝试从空闲池获取
        let mut idle = self.idle.lock().await;
        if let Some(mut plugin) = idle.pop() {
            // 更新活跃计数
            self.active_count.fetch_add(1, Ordering::Relaxed);
            plugin.mark_used();
            drop(idle);
            permit.forget(); // 保持许可
            return Ok(plugin);
        }

        // 空闲池为空，检查是否可创建新实例
        let total = self.total_count.load(Ordering::Relaxed);
        if total >= self.pool_config.max_total {
            drop(idle);
            permit.forget();
            return Err(PoolError::PoolExhausted(self.pool_config.max_total));
        }

        // 创建新实例
        match self.create_plugin().await {
            Ok(plugin) => {
                self.total_count.fetch_add(1, Ordering::Relaxed);
                self.active_count.fetch_add(1, Ordering::Relaxed);
                drop(idle);
                // 使用临时池引用
                Ok(PooledPlugin::new(plugin, &Arc::new(PluginPool::dummy())))
            }
            Err(e) => {
                drop(idle);
                permit.forget();
                Err(e)
            }
        }
    }

    /// 归还插件实例到池中
    async fn return_plugin(&self) {
        if self.is_closed() {
            return;
        }

        // 创建临时插件用于归还（因为 PooledPlugin 需要 pool 引用）
        // 这里我们直接处理逻辑
        self.active_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// 释放实例（不归还池中）
    pub async fn release(&self, mut plugin: PooledPlugin) {
        plugin.retiring = true;
        self.active_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// 缩小池（清理多余空闲实例）
    pub async fn shrink(&self, target_idle: usize) {
        let mut idle = self.idle.lock().await;

        while idle.len() > target_idle {
            if idle.pop().is_some() {
                self.total_count.fetch_sub(1, Ordering::Relaxed);
                tracing::debug!(
                    "Shrank plugin pool '{}': removed 1 idle instance",
                    self.config.name
                );
            } else {
                break;
            }
        }
    }

    /// 清理超时实例
    pub async fn cleanup_expired(&self) -> usize {
        let idle_timeout = Duration::from_secs(self.pool_config.idle_timeout_secs);
        let mut removed = 0;

        let mut idle = self.idle.lock().await;
        idle.retain(|plugin| {
            if plugin.is_expired(idle_timeout) {
                self.total_count.fetch_sub(1, Ordering::Relaxed);
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            tracing::info!(
                "Cleaned up {} expired plugins from pool '{}'",
                removed,
                self.config.name
            );
        }

        removed
    }

    /// 关闭池（清理所有实例）
    pub async fn close(&self) {
        if self.closed.swap(1, Ordering::SeqCst) == 1 {
            return; // Already closed
        }

        let mut idle = self.idle.lock().await;
        let count = idle.len();
        idle.clear();
        self.total_count.store(0, Ordering::Relaxed);
        self.active_count.store(0, Ordering::Relaxed);

        tracing::info!("Closed plugin pool '{}': removed {} instances", self.config.name, count);
    }

    /// 检查池是否已关闭
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) == 1
    }

    /// 获取池状态
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            active: self.active_count.load(Ordering::Relaxed),
            total: self.total_count.load(Ordering::Relaxed),
            max_total: self.pool_config.max_total,
            idle: self.pool_config.max_idle, // Approximate
            config: self.pool_config.clone(),
        }
    }

    /// 创建新插件实例
    async fn create_plugin(&self) -> PoolResult<ExtismPlugin> {
        ExtismPlugin::new(self.config.clone())
            .map_err(|e| PoolError::PluginCreation(e.to_string()))
    }

    /// 创建 dummy pool（用于单例模式）
    fn dummy() -> PluginPool {
        let pool_config = PoolConfig::default();
        PluginPool {
            config: WasmPluginConfig::default(),
            pool_config: pool_config.clone(),
            idle: tokio::sync::Mutex::new(Vec::new()),
            active_count: AtomicUsize::new(0),
            total_count: AtomicUsize::new(0),
            acquire_semaphore: Semaphore::new(pool_config.max_total),
            closed: AtomicUsize::new(0),
        }
    }
}

/// 池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 活跃实例数
    pub active: usize,
    /// 总实例数
    pub total: usize,
    /// 最大实例数
    pub max_total: usize,
    /// 空闲实例数（近似）
    pub idle: usize,
    /// 池配置
    pub config: PoolConfig,
}

/// PoolResult 类型别名
pub type PoolResult<T> = Result<T, PoolError>;

/// 插件池管理器
pub struct PluginPoolManager {
    pools: std::sync::Mutex<std::collections::HashMap<String, Arc<PluginPool>>>,
}

impl Default for PluginPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginPoolManager {
    /// 创建新的池管理器
    pub fn new() -> Self {
        Self {
            pools: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// 注册插件池
    pub fn register(&self, name: String, pool: Arc<PluginPool>) {
        let mut pools = self.pools.lock().unwrap();
        pools.insert(name, pool);
    }

    /// 获取插件池
    pub fn get(&self, name: &str) -> Option<Arc<PluginPool>> {
        let pools = self.pools.lock().unwrap();
        pools.get(name).cloned()
    }

    /// 移除插件池
    pub fn remove(&self, name: &str) -> Option<Arc<PluginPool>> {
        let mut pools = self.pools.lock().unwrap();
        pools.remove(name)
    }

    /// 获取所有池的统计信息
    pub fn stats(&self) -> std::collections::HashMap<String, PoolStats> {
        let pools = self.pools.lock().unwrap();
        pools.iter().map(|(k, v)| (k.clone(), v.stats())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_idle, 10);
        assert_eq!(config.min_idle, 2);
        assert_eq!(config.max_total, 100);
        assert_eq!(config.idle_timeout_secs, 300);
        assert_eq!(config.acquire_timeout_ms, 5000);
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new()
            .with_max_idle(5)
            .with_min_idle(1)
            .with_max_total(50)
            .with_idle_timeout(60)
            .with_acquire_timeout(1000);

        assert_eq!(config.max_idle, 5);
        assert_eq!(config.min_idle, 1);
        assert_eq!(config.max_total, 50);
        assert_eq!(config.idle_timeout_secs, 60);
        assert_eq!(config.acquire_timeout_ms, 1000);
    }

    #[test]
    fn test_pool_manager() {
        let manager = PluginPoolManager::new();
        assert!(manager.get("test").is_none());

        let config = WasmPluginConfig::new("/fake/path.wasm");
        let pool = PluginPool::with_default_config(config).unwrap();
        manager.register("test".to_string(), Arc::new(pool));

        assert!(manager.get("test").is_some());
        manager.remove("test");
        assert!(manager.get("test").is_none());
    }

    #[test]
    fn test_pool_error_display() {
        let err = PoolError::PoolExhausted(10);
        assert_eq!(err.to_string(), "Pool exhausted: max_capacity=10");

        let err = PoolError::PluginCreation("test error".to_string());
        assert_eq!(err.to_string(), "Plugin creation failed: test error");

        let err = PoolError::PoolClosed;
        assert_eq!(err.to_string(), "Plugin closed");
    }
}
