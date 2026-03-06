//! MemPlugin - EVIF Plugin for Memory Platform
//!
//! This plugin exposes the memory platform as a filesystem mount point,
//! allowing standard filesystem operations to interact with memory items.

#[cfg(feature = "plugin")]
pub mod plugin;
