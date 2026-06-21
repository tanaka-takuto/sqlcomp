use std::path::{Path, PathBuf};

use sqlay_app::CONFIG_FILE_NAME;
use sqlay_core as core;

use super::diagnostics::single_error_report;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ConfigSource {
    ExplicitPath(PathBuf),
    DiscoverFromCurrentDir,
    DiscoverFrom(PathBuf),
}

pub(super) fn resolve_path(source: &ConfigSource) -> core::DiagnosticResult<PathBuf> {
    match source {
        ConfigSource::ExplicitPath(path) => Ok(path.clone()),
        ConfigSource::DiscoverFromCurrentDir => {
            let start_dir = std::env::current_dir().map_err(|error| {
                single_error_report(
                    format!(
                        "failed to determine current directory while searching for `{CONFIG_FILE_NAME}`: {error}"
                    ),
                    None,
                )
            })?;

            discover_config_path(&start_dir)
        }
        ConfigSource::DiscoverFrom(start_dir) => discover_config_path(start_dir),
    }
}

fn discover_config_path(start_dir: &Path) -> core::DiagnosticResult<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Ok(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    Err(single_error_report(
        format!(
            "failed to find `{CONFIG_FILE_NAME}` from `{}` or any parent directory",
            start_dir.display()
        ),
        None,
    ))
}
