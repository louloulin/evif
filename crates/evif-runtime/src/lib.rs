// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 运行时 - 核心编排和配置管理

pub mod config;
pub mod runtime;
pub mod error;

pub use config::{RuntimeConfig, StorageConfig, AuthPolicy};
pub use runtime::EvifRuntime;
pub use error::{RuntimeError, RuntimeResult};
