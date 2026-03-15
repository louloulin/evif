// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 运行时 - 核心编排和配置管理

pub mod config;
pub mod error;
pub mod runtime;

pub use config::{AuthPolicy, RuntimeConfig, StorageConfig};
pub use error::{RuntimeError, RuntimeResult};
pub use runtime::EvifRuntime;
