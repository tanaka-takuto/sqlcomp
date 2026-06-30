use serde_json::{Map, Value};
use sqlay_core as core;

use super::super::diagnostics::{push_error, push_missing_field};

const SUPPORTED_CORE_TYPE_KEYS: [&str; 12] = [
    "bool", "int32", "int64", "float64", "decimal", "string", "bytes", "date", "time", "datetime",
    "json", "unknown",
];

pub(in crate::config_jsonc) fn validate_type_override_value(
    value: Value,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TypeScriptTypeOverride> {
    match value {
        Value::String(type_name) => {
            validate_type_name(&type_name, path, location, diagnostics)?;
            Some(core::TypeScriptTypeOverride::new(type_name, None))
        }
        Value::Object(map) => validate_type_override_object(map, path, location, diagnostics),
        _ => {
            push_error(
                diagnostics,
                format!(
                    "config field `{path}` must be a type name string or an object with `type` and optional `import`"
                ),
                location,
            );
            None
        }
    }
}

pub(in crate::config_jsonc) fn optional_object(
    raw: Option<Value>,
    path: &str,
    expected_shape: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<Map<String, Value>> {
    match raw {
        None => Some(Map::new()),
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            push_error(
                diagnostics,
                format!("config field `{path}` must be {expected_shape}"),
                location,
            );
            None
        }
    }
}

pub(in crate::config_jsonc) fn validate_column_reference(
    value: &str,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::ColumnTypeReference> {
    let parts = value.split('.').collect::<Vec<_>>();

    match parts.as_slice() {
        [table, column] if !table.is_empty() && !column.is_empty() => Some(
            core::ColumnTypeReference::new(None, (*table).to_owned(), (*column).to_owned()),
        ),
        [database, table, column]
            if !database.is_empty() && !table.is_empty() && !column.is_empty() =>
        {
            Some(core::ColumnTypeReference::new(
                Some((*database).to_owned()),
                (*table).to_owned(),
                (*column).to_owned(),
            ))
        }
        _ => {
            push_error(
                diagnostics,
                format!("config field `{path}` must use `table.column` or `database.table.column`"),
                location,
            );
            None
        }
    }
}

pub(in crate::config_jsonc) fn push_unknown_fields(
    map: &Map<String, Value>,
    path: &str,
    supported_fields: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) {
    for key in map.keys() {
        push_error(
            diagnostics,
            format!("unknown config field `{path}.{key}`; supported fields are {supported_fields}"),
            location,
        );
    }
}

pub(in crate::config_jsonc) fn core_type_from_config_key(value: &str) -> Option<core::CoreType> {
    match value {
        "bool" => Some(core::CoreType::Bool),
        "int32" => Some(core::CoreType::Int32),
        "int64" => Some(core::CoreType::Int64),
        "float64" => Some(core::CoreType::Float64),
        "decimal" => Some(core::CoreType::Decimal),
        "string" => Some(core::CoreType::String),
        "bytes" => Some(core::CoreType::Bytes),
        "date" => Some(core::CoreType::Date),
        "time" => Some(core::CoreType::Time),
        "datetime" => Some(core::CoreType::DateTime),
        "json" => Some(core::CoreType::Json),
        "unknown" => Some(core::CoreType::Unknown),
        _ => None,
    }
}

pub(in crate::config_jsonc) fn supported_core_type_keys_message() -> String {
    let (last, first) = SUPPORTED_CORE_TYPE_KEYS
        .split_last()
        .expect("supported core type keys should not be empty");
    format!(
        "{}, and `{last}`",
        first
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validate_type_override_object(
    mut map: Map<String, Value>,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TypeScriptTypeOverride> {
    let type_name = required_string_field(
        map.remove("type"),
        &format!("{path}.type"),
        location,
        diagnostics,
    )
    .and_then(|value| validate_type_name(&value, &format!("{path}.type"), location, diagnostics));
    let import = map.remove("import").and_then(|value| {
        validate_type_import(
            value,
            type_name.as_ref(),
            &format!("{path}.import"),
            location,
            diagnostics,
        )
    });

    push_unknown_fields(&map, path, "`type` and `import`", location, diagnostics);

    Some(core::TypeScriptTypeOverride::new(type_name?, import))
}

fn validate_type_import(
    value: Value,
    type_name: Option<&String>,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TypeScriptTypeImport> {
    let Value::Object(mut map) = value else {
        push_error(
            diagnostics,
            format!("config field `{path}` must be an object with `from` and `name`"),
            location,
        );
        return None;
    };

    let from = required_string_field(
        map.remove("from"),
        &format!("{path}.from"),
        location,
        diagnostics,
    )
    .and_then(|value| validate_import_from(&value, &format!("{path}.from"), location, diagnostics));
    let name = required_string_field(
        map.remove("name"),
        &format!("{path}.name"),
        location,
        diagnostics,
    )
    .and_then(|value| {
        validate_import_name(
            &value,
            type_name,
            &format!("{path}.name"),
            location,
            diagnostics,
        )
    });

    push_unknown_fields(&map, path, "`from` and `name`", location, diagnostics);

    Some(core::TypeScriptTypeImport::new(from?, name?))
}

fn required_string_field(
    value: Option<Value>,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<String> {
    match value {
        Some(Value::String(value)) => Some(value),
        Some(_) => {
            push_error(
                diagnostics,
                format!("config field `{path}` must be a string"),
                location,
            );
            None
        }
        None => {
            push_missing_field(diagnostics, path, location);
            None
        }
    }
}

fn validate_type_name(
    value: &str,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<String> {
    if is_portable_type_identifier(value) {
        Some(value.to_owned())
    } else {
        push_error(
            diagnostics,
            format!(
                "config field `{path}` value `{value}` must be a supported TypeScript primitive or portable type identifier matching `^[A-Za-z_][A-Za-z0-9_]*$`"
            ),
            location,
        );
        None
    }
}

fn validate_import_from(
    value: &str,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<String> {
    if is_non_relative_module_specifier(value) {
        Some(value.to_owned())
    } else {
        push_error(
            diagnostics,
            format!(
                "config field `{path}` value `{value}` must be a non-relative module specifier"
            ),
            location,
        );
        None
    }
}

fn validate_import_name(
    value: &str,
    type_name: Option<&String>,
    path: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<String> {
    validate_type_name(value, path, location, diagnostics)?;
    let expected_type_name = type_name.map(String::as_str);

    if expected_type_name == Some(value) {
        Some(value.to_owned())
    } else {
        let expected = expected_type_name.unwrap_or("<invalid>");
        push_error(
            diagnostics,
            format!(
                "config field `{path}` value `{value}` must match `type` value `{expected}`; import aliases are not supported"
            ),
            location,
        );
        None
    }
}

fn is_portable_type_identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == b'_')
        && chars.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_non_relative_module_specifier(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.starts_with("./")
        && !value.starts_with("../")
        && !value.starts_with('/')
}
