use std::path::{Path, PathBuf};

use serde_json::Value;
use sqlay_core as core;

use super::diagnostics::{
    parse_error_location, push_error, push_missing_field, single_error_report,
};
use super::jsonc::normalize_jsonc;
use super::raw::{
    RawDatabaseConfig, RawOutputConfig, RawProjectConfig, RawSourceConfig, RawTargetConfig,
    RawTypeScriptTargetConfig, RawTypeScriptTypeMappingConfig,
};
use super::type_mapping::{
    core_type_from_config_key, optional_object, push_unknown_fields,
    supported_core_type_keys_message, validate_column_reference, validate_type_override_value,
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
        .and_then(|value| validate_target_language(&value, location, diagnostics))?;
    let typescript = validate_typescript_target(raw.typescript, location, diagnostics)?;

    Some(
        core::TargetConfig::new(language)
            .with_typescript_type_mapping(typescript.type_mapping().clone()),
    )
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

fn validate_typescript_target(
    raw: Option<RawTypeScriptTargetConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TypeScriptTargetConfig> {
    let Some(raw) = raw else {
        return Some(core::TypeScriptTargetConfig::empty());
    };

    let type_mapping = raw.type_mapping.map_or_else(
        || Some(core::TypeScriptTypeMappingConfig::empty()),
        |type_mapping| validate_type_mapping(type_mapping, location, diagnostics),
    )?;

    Some(core::TypeScriptTargetConfig::new(type_mapping))
}

fn validate_type_mapping(
    raw: RawTypeScriptTypeMappingConfig,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TypeScriptTypeMappingConfig> {
    let core = validate_core_type_overrides(raw.core, location, diagnostics)?;
    let columns = validate_column_type_overrides(raw.columns, location, diagnostics)?;
    let builders = validate_builder_type_overrides(raw.builders, location, diagnostics)?;

    Some(core::TypeScriptTypeMappingConfig::new(
        core, columns, builders,
    ))
}

fn validate_core_type_overrides(
    raw: Option<Value>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Vec<core::CoreTypeOverride>> {
    let entries = optional_object(
        raw,
        "target.typescript.typeMapping.core",
        "an object keyed by Core type names",
        location,
        diagnostics,
    )?;
    let mut overrides = Vec::new();

    for (key, value) in entries {
        let path = format!("target.typescript.typeMapping.core.{key}");
        let Some(core_type) = core_type_from_config_key(&key) else {
            push_error(
                diagnostics,
                format!(
                    "unsupported config field `{path}`; supported core type keys are {}",
                    supported_core_type_keys_message()
                ),
                location,
            );
            continue;
        };

        if let Some(type_override) =
            validate_type_override_value(value, &path, location, diagnostics)
        {
            overrides.push(core::CoreTypeOverride::new(core_type, type_override));
        }
    }

    Some(overrides)
}

fn validate_column_type_overrides(
    raw: Option<Value>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Vec<core::ColumnTypeOverride>> {
    let entries = optional_object(
        raw,
        "target.typescript.typeMapping.columns",
        "an object keyed by `table.column` or `database.table.column`",
        location,
        diagnostics,
    )?;
    let mut overrides = Vec::new();

    for (key, value) in entries {
        let path = format!("target.typescript.typeMapping.columns.{key}");
        let Some(reference) = validate_column_reference(&key, &path, location, diagnostics) else {
            continue;
        };

        if let Some(type_override) =
            validate_type_override_value(value, &path, location, diagnostics)
        {
            overrides.push(core::ColumnTypeOverride::new(reference, type_override));
        }
    }

    Some(overrides)
}

fn validate_builder_type_overrides(
    raw: Option<Value>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Vec<core::BuilderTypeOverrides>> {
    let entries = optional_object(
        raw,
        "target.typescript.typeMapping.builders",
        "an object keyed by generated builder IDs",
        location,
        diagnostics,
    )?;
    let mut overrides = Vec::new();

    for (builder_id, value) in entries {
        let path = format!("target.typescript.typeMapping.builders.{builder_id}");
        if builder_id.is_empty() {
            push_error(
                diagnostics,
                "config field `target.typescript.typeMapping.builders` contains an empty key; keys must be non-empty",
                location,
            );
            continue;
        }

        if let Some(builder) =
            validate_builder_type_override(builder_id, value, &path, location, diagnostics)
        {
            overrides.push(builder);
        }
    }

    Some(overrides)
}

fn validate_builder_type_override(
    builder_id: String,
    value: Value,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::BuilderTypeOverrides> {
    let Value::Object(mut map) = value else {
        push_error(
            diagnostics,
            format!(
                "config field `{path}` must be an object with `fields`, `params`, or `repeats`"
            ),
            location,
        );
        return None;
    };

    let fields = validate_named_type_overrides(
        map.remove("fields"),
        &format!("{path}.fields"),
        "an object keyed by generated result field names",
        location,
        diagnostics,
    )?;
    let params = validate_named_type_overrides(
        map.remove("params"),
        &format!("{path}.params"),
        "an object keyed by generated Param names",
        location,
        diagnostics,
    )?;
    let repeats = validate_repeat_type_overrides(
        map.remove("repeats"),
        &format!("{path}.repeats"),
        location,
        diagnostics,
    )?;

    push_unknown_fields(
        &map,
        path,
        "`fields`, `params`, and `repeats`",
        location,
        diagnostics,
    );

    Some(core::BuilderTypeOverrides::new(
        builder_id, fields, params, repeats,
    ))
}

fn validate_repeat_type_overrides(
    raw: Option<Value>,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Vec<core::RepeatTypeOverrides>> {
    let entries = optional_object(
        raw,
        path,
        "an object keyed by generated Repeat input IDs",
        location,
        diagnostics,
    )?;
    let mut overrides = Vec::new();

    for (repeat_id, value) in entries {
        let repeat_path = format!("{path}.{repeat_id}");
        if repeat_id.is_empty() {
            push_error(
                diagnostics,
                format!("config field `{path}` contains an empty key; keys must be non-empty"),
                location,
            );
            continue;
        }

        if let Some(repeat) =
            validate_repeat_type_override(repeat_id, value, &repeat_path, location, diagnostics)
        {
            overrides.push(repeat);
        }
    }

    Some(overrides)
}

fn validate_repeat_type_override(
    repeat_id: String,
    value: Value,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::RepeatTypeOverrides> {
    let Value::Object(mut map) = value else {
        push_error(
            diagnostics,
            format!("config field `{path}` must be an object with `fields`"),
            location,
        );
        return None;
    };

    let fields = validate_named_type_overrides(
        map.remove("fields"),
        &format!("{path}.fields"),
        "an object keyed by generated Repeat item field names",
        location,
        diagnostics,
    )?;

    push_unknown_fields(&map, path, "`fields`", location, diagnostics);

    Some(core::RepeatTypeOverrides::new(repeat_id, fields))
}

fn validate_named_type_overrides(
    raw: Option<Value>,
    path: &str,
    expected_shape: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Vec<core::NamedTypeOverride>> {
    let entries = optional_object(raw, path, expected_shape, location, diagnostics)?;
    let mut overrides = Vec::new();

    for (name, value) in entries {
        let entry_path = format!("{path}.{name}");
        if name.is_empty() {
            push_error(
                diagnostics,
                format!("config field `{path}` contains an empty key; keys must be non-empty"),
                location,
            );
            continue;
        }

        if let Some(type_override) =
            validate_type_override_value(value, &entry_path, location, diagnostics)
        {
            overrides.push(core::NamedTypeOverride::new(name, type_override));
        }
    }

    Some(overrides)
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
