use async_trait::async_trait;
use evif_core::{
    validate_and_initialize_plugin, EvifError, EvifPlugin, EvifResult, FileInfo, RadixMountTable,
    WriteFlags,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default)]
struct LifecycleFlags {
    validated: AtomicBool,
    initialized: AtomicBool,
    shutdown: AtomicBool,
    fail_initialize: AtomicBool,
}

struct LifecyclePlugin {
    flags: Arc<LifecycleFlags>,
}

impl LifecyclePlugin {
    fn new(flags: Arc<LifecycleFlags>) -> Self {
        Self { flags }
    }
}

#[async_trait]
impl EvifPlugin for LifecyclePlugin {
    fn name(&self) -> &str {
        "lifecycle"
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn read(&self, _path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn write(
        &self,
        _path: &str,
        _data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn readdir(&self, _path: &str) -> EvifResult<Vec<FileInfo>> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn stat(&self, _path: &str) -> EvifResult<FileInfo> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupportedGeneric)
    }

    async fn validate(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        self.flags.validated.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn initialize(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        self.flags.initialized.store(true, Ordering::SeqCst);
        if self.flags.fail_initialize.load(Ordering::SeqCst) {
            return Err(EvifError::Configuration("init failed".to_string()));
        }
        Ok(())
    }

    async fn shutdown(&self) -> EvifResult<()> {
        self.flags.shutdown.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn test_validate_and_initialize_plugin_calls_both_hooks() {
    let flags = Arc::new(LifecycleFlags::default());
    let plugin = LifecyclePlugin::new(flags.clone());

    validate_and_initialize_plugin(&plugin, None).await.unwrap();

    assert!(flags.validated.load(Ordering::SeqCst));
    assert!(flags.initialized.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_validate_and_initialize_plugin_returns_initialize_error() {
    let flags = Arc::new(LifecycleFlags::default());
    flags.fail_initialize.store(true, Ordering::SeqCst);
    let plugin = LifecyclePlugin::new(flags.clone());

    let err = validate_and_initialize_plugin(&plugin, None)
        .await
        .expect_err("initialize should fail");
    assert!(matches!(err, EvifError::Configuration(_)));
    assert!(flags.validated.load(Ordering::SeqCst));
    assert!(flags.initialized.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_unmount_calls_plugin_shutdown() {
    let flags = Arc::new(LifecycleFlags::default());
    let plugin = Arc::new(LifecyclePlugin::new(flags.clone())) as Arc<dyn EvifPlugin>;
    let mount_table = RadixMountTable::new();

    mount_table.mount("/lifecycle".to_string(), plugin).await.unwrap();
    mount_table.unmount("/lifecycle").await.unwrap();

    assert!(flags.shutdown.load(Ordering::SeqCst));
}

