use serde_json::{Map, Value};
use sqlay_core as core;

use crate::source_fs::diagnostics::metadata_error;
use crate::source_fs::scanner::SqlayBlock;

const SUPPORTED_PARAM_VALUE_TYPES: [&str; 11] = [
    "bool", "int32", "int64", "float64", "decimal", "string", "bytes", "date", "time", "datetime",
    "json",
];

pub(super) fn reject_unknown_metadata_fields(
    metadata: &Map<String, Value>,
    allowed_fields: &[&str],
    annotation_type: &str,
    supported_fields: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<()> {
    if let Some(field) = metadata
        .keys()
        .find(|field| !allowed_fields.contains(&field.as_str()))
    {
        return Err(metadata_error(
            format!(
                "unknown `{annotation_type}` metadata field `{field}`; supported fields are {supported_fields}"
            ),
            block.payload_range(),
        ));
    }

    Ok(())
}

pub(super) fn required_annotation_type_from_metadata(
    metadata: &Map<String, Value>,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    match metadata.get("type") {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(metadata_error(
            "`@sqlay` metadata field `type` must be a string",
            block.payload_range(),
        )),
        None => Err(metadata_error(
            "missing required `@sqlay` metadata field `type`",
            block.payload_range(),
        )),
    }
}

pub(super) fn required_param_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    required_string_metadata_field(metadata, field, "param", block)
}

pub(super) fn required_fragment_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    required_string_metadata_field(metadata, field, "fragment", block)
}

pub(super) fn required_mutation_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    required_string_metadata_field(metadata, field, "mutation", block)
}

pub(super) fn required_slot_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    required_string_metadata_field(metadata, field, "slot", block)
}

pub(super) fn required_repeat_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    required_string_metadata_field(metadata, field, "repeat", block)
}

fn required_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    annotation: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<String> {
    match metadata.get(field) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(metadata_error(
            format!("`{annotation}` metadata field `{field}` must be a string"),
            block.payload_range(),
        )),
        None => Err(metadata_error(
            format!("missing required `{annotation}` metadata field `{field}`"),
            block.payload_range(),
        )),
    }
}

pub(super) fn required_slot_targets_metadata_field(
    metadata: &Map<String, Value>,
    block: &SqlayBlock,
) -> core::DiagnosticResult<Vec<String>> {
    let Some(targets) = metadata.get("targets") else {
        return Err(metadata_error(
            "missing required `slot` metadata field `targets`",
            block.payload_range(),
        ));
    };
    let Value::Array(values) = targets else {
        return Err(metadata_error(
            "`slot` metadata field `targets` must be a string array",
            block.payload_range(),
        ));
    };
    if values.is_empty() {
        return Err(metadata_error(
            "`slot` metadata field `targets` must contain at least one value",
            block.payload_range(),
        ));
    }

    values
        .iter()
        .map(|value| match value {
            Value::String(target) if is_valid_query_id(target) => Ok(target.clone()),
            Value::String(target) => Err(metadata_error(
                format!("invalid Slot target `{target}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
                block.payload_range(),
            )),
            _ => Err(metadata_error(
                "`slot` metadata field `targets` must be a string array",
                block.payload_range(),
            )),
        })
        .collect()
}

pub(super) fn optional_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlayBlock,
) -> core::DiagnosticResult<Option<String>> {
    match metadata.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(metadata_error(
            format!("`@sqlay` metadata field `{field}` must be a string"),
            block.payload_range(),
        )),
        None => Ok(None),
    }
}

pub(super) fn validate_param_nullable(
    nullable: Option<bool>,
    block: &SqlayBlock,
) -> core::DiagnosticResult<bool> {
    match nullable {
        Some(true) => Ok(true),
        None => Ok(false),
        Some(false) => Err(metadata_error(
            "`nullable: false` is not supported for Param metadata; omit `nullable` for non-null inputs",
            block.payload_range(),
        )),
    }
}

pub(super) fn parse_param_value_type(
    value_type: Option<&str>,
    block: &SqlayBlock,
) -> core::DiagnosticResult<Option<core::CoreType>> {
    let Some(value_type) = value_type else {
        return Ok(None);
    };

    if value_type.is_empty() {
        return Err(metadata_error(
            "`param` metadata field `valueType` must not be empty",
            block.payload_range(),
        ));
    }
    if let Some(nullable_value_type) = nullable_union_param_value_type(value_type) {
        return Err(metadata_error(
            format!(
                "unsupported Param valueType `{value_type}`; use `valueType: {nullable_value_type}` with `nullable: true` for nullable {nullable_value_type} inputs; optional input properties are not supported"
            ),
            block.payload_range(),
        ));
    }
    if !SUPPORTED_PARAM_VALUE_TYPES.contains(&value_type) {
        return Err(metadata_error(
            format!(
                "unsupported Param valueType `{value_type}`; supported values are {}",
                supported_param_value_types_message()
            ),
            block.payload_range(),
        ));
    }

    Ok(Some(core_type_from_param_value_type(value_type).expect(
        "supported Param valueType should map to a CoreType",
    )))
}

fn nullable_union_param_value_type(value_type: &str) -> Option<&str> {
    let (base, nullable) = value_type.split_once('|')?;
    if nullable.trim() != "null" {
        return None;
    }

    let base = base.trim();
    SUPPORTED_PARAM_VALUE_TYPES.contains(&base).then_some(base)
}

fn core_type_from_param_value_type(value_type: &str) -> Option<core::CoreType> {
    match value_type.as_bytes() {
        b"bool" => Some(core::CoreType::Bool),
        b"int32" => Some(core::CoreType::Int32),
        b"int64" => Some(core::CoreType::Int64),
        b"float64" => Some(core::CoreType::Float64),
        b"decimal" => Some(core::CoreType::Decimal),
        b"string" => Some(core::CoreType::String),
        b"bytes" => Some(core::CoreType::Bytes),
        b"date" => Some(core::CoreType::Date),
        b"time" => Some(core::CoreType::Time),
        b"datetime" => Some(core::CoreType::DateTime),
        b"json" => Some(core::CoreType::Json),
        _ => None,
    }
}

fn supported_param_value_types_message() -> String {
    let (last, first) = SUPPORTED_PARAM_VALUE_TYPES
        .split_last()
        .expect("Param valueType list is non-empty");

    format!(
        "{}, and `{last}`",
        first
            .iter()
            .map(|value_type| format!("`{value_type}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn parse_cardinality(
    raw_cardinality: Option<String>,
    block: &SqlayBlock,
) -> core::DiagnosticResult<Option<core::Cardinality>> {
    let Some(raw_cardinality) = raw_cardinality else {
        return Ok(None);
    };

    match raw_cardinality.as_str() {
        "one" => Ok(Some(core::Cardinality::One)),
        "many" => Ok(Some(core::Cardinality::Many)),
        "exec" => Err(metadata_error(
            "`cardinality: exec` is reserved for future non-SELECT support and is not currently supported",
            block.payload_range(),
        )),
        _ => Err(metadata_error(
            format!(
                "unsupported query cardinality `{raw_cardinality}`; supported values are `one` and `many`"
            ),
            block.payload_range(),
        )),
    }
}

pub(super) fn is_valid_query_id(id: &str) -> bool {
    let mut bytes = id.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };

    is_query_id_start(first) && bytes.all(is_query_id_continue)
}

const fn is_query_id_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

const fn is_query_id_continue(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}
