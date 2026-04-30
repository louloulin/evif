// EVIF Plugins - AGFS 插件实现
//
// 完全对标 AGFS 的插件集合

pub mod catalog;
pub mod contextfs;
pub mod context_manager;
pub mod pipefs;
pub mod localfs;
pub mod kvfs;
pub mod queuefs;
pub mod skillfs;
pub mod serverinfofs;
pub mod memfs;
pub mod httpfs;
pub mod streamfs;
pub mod proxyfs;
pub mod devfs;
pub mod hellofs;
pub mod heartbeatfs;
pub mod handlefs;
pub mod tieredfs;
pub mod encryptedfs;

#[cfg(feature = "s3fs")]
pub mod s3fs;

#[cfg(feature = "sqlfs")]
pub mod sqlfs;

#[cfg(feature = "gptfs")]
pub mod gptfs;

#[cfg(feature = "vectorfs")]
pub mod vectorfs;

// SQLFS2 - Always available (simplified version without external dependencies)
// pub mod sqlfs2_simple;  // 暂时禁用,需要修复

#[cfg(feature = "streamrotatefs")]
pub mod streamrotatefs;

// Skill Runtime Safety Layer (Phase 9.1 - SkillFS)
pub mod skill_runtime;

// Exports from skill_runtime module
pub use skill_runtime::{
    execute_skill, execute_skill_with_timeout, get_runtime_info, validate_skill_execution,
    SkillExecutionContext, SkillExecutionResult, SkillExecutor, SkillRuntimeError,
};

// OpenDAL-based plugins (EVIF 2.1)
#[cfg(feature = "opendal")]
pub mod opendal;

#[cfg(feature = "opendal")]
pub mod s3fs_opendal;

#[cfg(feature = "azureblobfs")]
pub mod azureblobfs;

#[cfg(feature = "gcsfs")]
pub mod gcsfs;

#[cfg(feature = "aliyunossfs")]
pub mod aliyunossfs;

#[cfg(feature = "tencentcosfs")]
pub mod tencentcosfs;

#[cfg(feature = "huaweiobsfs")]
pub mod huaweiobsfs;

#[cfg(feature = "miniofs")]
pub mod miniofs;
#[cfg(feature = "webdavfs")]
pub mod webdavfs;
#[cfg(feature = "ftpfs")]
pub mod ftpfs;
#[cfg(feature = "sftpfs")]
pub mod sftpfs;

pub use localfs::LocalFsPlugin;
pub use contextfs::{ContextFsPlugin, ContextTokenBudget, BudgetLevel, BudgetStatus};
pub use context_manager::{ContextManager, ContextLayer, SessionInfo, SearchResult, SemanticResult};
pub use pipefs::PipeFsPlugin;
pub use kvfs::KvfsPlugin;
pub use queuefs::QueueFsPlugin;
pub use skillfs::SkillFsPlugin;
pub use skillfs::{validate_skill_md, SkillMetadata, SkillValidationError};
pub use serverinfofs::ServerInfoFsPlugin;
pub use memfs::MemFsPlugin;
pub use httpfs::HttpFsPlugin;
pub use streamfs::StreamFsPlugin;
pub use proxyfs::ProxyFsPlugin;
pub use devfs::DevFsPlugin;
pub use hellofs::HelloFsPlugin;
pub use heartbeatfs::{HeartbeatFsPlugin, HeartbeatConfig};
pub use handlefs::{HandleFsPlugin, FileHandle, OpenFlags, HandleFsConfig};
pub use tieredfs::{TieredFsPlugin, TieredConfig, StorageTier, TierStats};
pub use encryptedfs::{EncryptedFsPlugin, EncryptedConfig};
pub use catalog::{
    core_supported_plugins, experimental_plugins, find_plugin_catalog_entry, normalize_plugin_id,
    plugin_catalog, PluginCatalogEntry, PluginSupportTier,
};

#[cfg(feature = "s3fs")]
pub use s3fs::{S3fsPlugin, S3Config, DirCache, StatCache};

#[cfg(feature = "sqlfs")]
pub use sqlfs::{SqlfsPlugin, SqlfsConfig, MAX_FILE_SIZE};

#[cfg(feature = "gptfs")]
pub use gptfs::{GptfsPlugin, GptfsConfig, Job, JobStatus};

#[cfg(feature = "vectorfs")]
pub use vectorfs::{VectorFsPlugin, VectorFsConfig, EmbeddingProvider, OpenAIEmbeddingProvider, NoEmbeddingProvider};

// SQLFS2 - Always available
// pub use sqlfs2_simple::{create_sqlfs2_plugin, SqlFS2Plugin};

#[cfg(feature = "streamrotatefs")]
pub use streamrotatefs::{StreamRotateFSPlugin, RotationConfig};

// OpenDAL-based plugins (EVIF 2.1)
#[cfg(feature = "opendal")]
pub use opendal::{OpendalPlugin, OpendalConfig, OpendalService};

#[cfg(feature = "opendal")]
pub use s3fs_opendal::{S3FsPlugin, S3Config as S3ConfigOpenDAL};

#[cfg(feature = "azureblobfs")]
pub use azureblobfs::{AzureBlobFsPlugin, AzureBlobConfig};

#[cfg(feature = "gcsfs")]
pub use gcsfs::{GcsFsPlugin, GcsConfig};

#[cfg(feature = "aliyunossfs")]
pub use aliyunossfs::{AliyunOssFsPlugin, AliyunOssConfig};

#[cfg(feature = "tencentcosfs")]
pub use tencentcosfs::{TencentCosFsPlugin, TencentCosConfig};

#[cfg(feature = "huaweiobsfs")]
pub use huaweiobsfs::{HuaweiObsFsPlugin, HuaweiObsConfig};

#[cfg(feature = "miniofs")]
pub use miniofs::{MinioFsPlugin, MinioConfig};
#[cfg(feature = "webdavfs")]
pub use webdavfs::{WebdavFsPlugin, WebdavConfig};
#[cfg(feature = "ftpfs")]
pub use ftpfs::{FtpFsPlugin, FtpConfig};
#[cfg(feature = "sftpfs")]
pub use sftpfs::{SftpFsPlugin, SftpConfig};
