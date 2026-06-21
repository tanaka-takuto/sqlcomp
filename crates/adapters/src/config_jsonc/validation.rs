use std::path::{Path, PathBuf};

use sqlay_core as core;

use super::diagnostics::{
    parse_error_location, push_error, push_missing_field, single_error_report,
};
use super::jsonc::normalize_jsonc;
use super::raw::{
    RawDatabaseConfig, RawOutputConfig, RawProjectConfig, RawSourceConfig, RawTargetConfig,
};

pub(super) fn parse_config(
    source: &str,
    path: Option<&Path>,
    config_dir: PathBuf,
) -> core::DiagnosticResult<core::ProjectConfig> {
    let normalized = normalize_jsonc(source).map_err(|message| {
        single_error_report(
            format!("failed to parse `sqlay.config.json` as JSONC: {message}"),
            path.map(core::SourceLocation::for_path),
        )
    })?;

    let raw = serde_json::from_str::<RawProjectConfig>(&normalized).map_err(|error| {
        let location = parse_error_location(path, &error);
        single_error_report(
            format!("failed to parse `sqlay.config.json` as JSONC: {error}"),
            location,
        )
    })?;

    validate_config(raw, path, config_dir)
}

fn validate_config(
    raw: RawProjectConfig,
    path: Option<&Path>,
    config_dir: PathBuf,
) -> core::DiagnosticResult<core::ProjectConfig> {
    let location = path.map(core::SourceLocation::for_path);
    let mut diagnostics = core::DiagnosticReport::default();

    let source = validate_source(raw.source, location.as_ref(), &mut diagnostics);
    let output = validate_output(raw.output, location.as_ref(), &mut diagnostics);
    let database = validate_database(raw.database, location.as_ref(), &mut diagnostics);
    let target = validate_target(raw.target, location.as_ref(), &mut diagnostics);

    if diagnostics.is_empty() {
        if let (Some(source), Some(output), Some(database), Some(target)) =
            (source, output, database, target)
        {
            Ok(core::ProjectConfig::new(
                config_dir, source, output, database, target,
            ))
        } else {
            Err(diagnostics)
        }
    } else {
        Err(diagnostics)
    }
}

fn validate_source(
    raw: Option<RawSourceConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::SourceConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "source.include", location);
        return None;
    };

    let include = required_field(raw.include, "source.include", location, diagnostics)?;
    let exclude = raw.exclude.unwrap_or_default();

    Some(core::SourceConfig::new(include, exclude))
}

fn validate_output(
    raw: Option<RawOutputConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::OutputConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "output.dir", location);
        return None;
    };

    let dir = required_field(raw.dir, "output.dir", location, diagnostics)?;

    Some(core::OutputConfig::new(dir))
}

fn validate_database(
    raw: Option<RawDatabaseConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::DatabaseConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "database.dialect", location);
        push_missing_field(diagnostics, "database.urlEnv", location);
        return None;
    };

    let dialect = required_field(raw.dialect, "database.dialect", location, diagnostics)
        .and_then(|value| validate_database_dialect(&value, location, diagnostics));
    let url_env = required_field(raw.url_env, "database.urlEnv", location, diagnostics);

    Some(core::DatabaseConfig::new(dialect?, url_env?))
}

fn validate_target(
    raw: Option<RawTargetConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TargetConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "target.language", location);
        return None;
    };

    let language = required_field(raw.language, "target.language", location, diagnostics)
        .and_then(|value| validate_target_language(&value, location, diagnostics));

    Some(core::TargetConfig::new(language?))
}

fn validate_database_dialect(
    value: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::DatabaseDialect> {
    if value == "mysql" {
        Some(core::DatabaseDialect::MySql)
    } else {
        push_error(
            diagnostics,
            format!(
                "unsupported config field `database.dialect` value `{value}`; supported value is `mysql`"
            ),
            location,
        );
        None
    }
}

fn validate_target_language(
    value: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TargetLanguage> {
    if value == "typescript" {
        Some(core::TargetLanguage::TypeScript)
    } else {
        push_error(
            diagnostics,
            format!(
                "unsupported config field `target.language` value `{value}`; supported value is `typescript`"
            ),
            location,
        );
        None
    }
}

fn required_field<T>(
    value: Option<T>,
    name: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<T> {
    if value.is_none() {
        push_missing_field(diagnostics, name, location);
    }

    value
}
