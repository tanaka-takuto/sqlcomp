//! JSONC configuration adapter.

mod diagnostics;
mod jsonc;
mod loader;
mod paths;
mod raw;
#[cfg(test)]
mod tests;
mod validation;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sqlcomp_app::{ConfigLoader, ConfigTemplateWriter};
use sqlcomp_core as core;

use diagnostics::single_error_report;
use loader::{ConfigSource, resolve_path};
use paths::config_dir_from_path;
use validation::parse_config;

/// JSONC-backed config loader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsoncConfigLoader {
    source: ConfigSource,
}

/// Filesystem-backed starter config writer.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsoncConfigTemplateWriter;

impl JsoncConfigLoader {
    /// Build a loader for an explicit config file path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ConfigSource::ExplicitPath(path.into()),
        }
    }

    /// Build a loader that discovers `sqlcomp.config.json` from the process
    /// current directory upward.
    #[must_use]
    pub const fn discover_from_current_dir() -> Self {
        Self {
            source: ConfigSource::DiscoverFromCurrentDir,
        }
    }

    /// Build a loader that discovers `sqlcomp.config.json` from a directory
    /// upward.
    #[must_use]
    pub fn discover_from(start_dir: impl Into<PathBuf>) -> Self {
        Self {
            source: ConfigSource::DiscoverFrom(start_dir.into()),
        }
    }

    /// Return the explicit path this loader reads when one was configured.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match &self.source {
            ConfigSource::ExplicitPath(path) => Some(path),
            ConfigSource::DiscoverFromCurrentDir | ConfigSource::DiscoverFrom(_) => None,
        }
    }

    /// Parse and validate JSONC configuration content.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the content cannot be parsed as JSONC or when
    /// required fields are missing or unsupported.
    pub fn parse_str(source: &str) -> core::DiagnosticResult<core::ProjectConfig> {
        parse_config(source, None, Path::new(".").to_path_buf())
    }

    /// Parse and validate JSONC configuration content with an explicit config
    /// directory.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the content cannot be parsed as JSONC or when
    /// required fields are missing or unsupported.
    pub fn parse_str_from_dir(
        source: &str,
        config_dir: impl Into<PathBuf>,
    ) -> core::DiagnosticResult<core::ProjectConfig> {
        parse_config(source, None, config_dir.into())
    }
}

impl Default for JsoncConfigLoader {
    fn default() -> Self {
        Self::discover_from_current_dir()
    }
}

impl ConfigLoader for JsoncConfigLoader {
    fn load(&self) -> core::DiagnosticResult<core::ProjectConfig> {
        let path = resolve_path(&self.source)?;
        let source = fs::read_to_string(&path).map_err(|error| {
            single_error_report(
                format!("failed to read config file `{}`: {error}", path.display()),
                Some(core::SourceLocation::for_path(path.clone())),
            )
        })?;

        parse_config(&source, Some(&path), config_dir_from_path(&path))
    }
}

impl ConfigTemplateWriter for JsoncConfigTemplateWriter {
    fn write_new(&self, path: &Path, contents: &str) -> core::DiagnosticResult<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    single_error_report(
                        format!(
                            "refusing to overwrite existing config file `{}`",
                            path.display()
                        ),
                        Some(core::SourceLocation::for_path(path)),
                    )
                } else {
                    single_error_report(
                        format!("failed to create config file `{}`: {error}", path.display()),
                        Some(core::SourceLocation::for_path(path)),
                    )
                }
            })?;

        file.write_all(contents.as_bytes()).map_err(|error| {
            single_error_report(
                format!("failed to write config file `{}`: {error}", path.display()),
                Some(core::SourceLocation::for_path(path)),
            )
        })
    }
}
