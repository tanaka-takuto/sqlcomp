use std::path::{Path, PathBuf};

use sqlcomp_core as core;

use crate::{CONFIG_FILE_NAME, ConfigTemplateWriter, STARTER_CONFIG_TEMPLATE};

/// Application service for initializing a sqlcomp project config.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultProjectInitializer;

impl DefaultProjectInitializer {
    /// Create the starter config in `current_dir`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the config file already exists or cannot be
    /// written.
    pub fn init(
        current_dir: &Path,
        writer: &impl ConfigTemplateWriter,
    ) -> core::DiagnosticResult<PathBuf> {
        let config_path = current_dir.join(CONFIG_FILE_NAME);
        writer.write_new(&config_path, STARTER_CONFIG_TEMPLATE)?;

        Ok(config_path)
    }
}
