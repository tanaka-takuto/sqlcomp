//! JSONC configuration adapter.

use sqlcomp_app::ConfigLoader;
use sqlcomp_core as core;

/// Dummy JSONC-backed config loader.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsoncConfigLoader;

impl ConfigLoader for JsoncConfigLoader {
    fn load(&self) -> core::ProjectConfig {
        core::ProjectConfig
    }
}
