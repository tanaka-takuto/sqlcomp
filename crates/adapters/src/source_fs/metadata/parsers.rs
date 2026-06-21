use serde::Deserialize;
use serde_json::{Map, Value};
use sqlcomp_core as core;

use crate::source_fs::diagnostics::metadata_error;
use crate::source_fs::metadata::fields::{
    is_valid_query_id, optional_string_metadata_field, parse_cardinality, parse_param_value_type,
    reject_unknown_metadata_fields, required_annotation_type_from_metadata,
    required_fragment_string_metadata_field, required_param_string_metadata_field,
    required_slot_string_metadata_field, required_slot_targets_metadata_field,
    validate_param_nullable,
};
use crate::source_fs::metadata::hjson::{
    deserialize_sqlcomp_metadata, parse_sqlcomp_metadata_object,
};
use crate::source_fs::scanner::SqlcompBlock;

/// Parse one discovered `@sqlcomp` block as query metadata.
///
/// # Errors
///
/// Returns diagnostics when the payload is malformed Hjson, does not declare a
/// query annotation, or contains invalid query metadata.
pub fn parse_sqlcomp_query_metadata(
    block: &SqlcompBlock,
) -> core::DiagnosticResult<core::QueryMetadata> {
    let raw = deserialize_sqlcomp_metadata::<RawSqlcompQueryMetadata>(block)?;
    let Some(annotation_type) = raw.annotation_type.as_deref() else {
        return Err(metadata_error(
            "missing required `@sqlcomp` metadata field `type`",
            block.payload_range(),
        ));
    };

    if annotation_type != "query" {
        return Err(metadata_error(
            format!(
                "unsupported `@sqlcomp` annotation type `{annotation_type}`; expected `query` metadata"
            ),
            block.payload_range(),
        ));
    }

    parse_query_metadata(raw, block)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::source_fs) enum SqlcompAnnotation {
    Query(core::QueryMetadata),
    Fragment(core::FragmentMetadata),
    Param(ParsedParamMetadata),
    ParamEnd,
    Slot(ParsedSlotMetadata),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::source_fs) struct ParsedParamMetadata {
    pub(in crate::source_fs) id: String,
    pub(in crate::source_fs) value_type: Option<core::CoreType>,
    pub(in crate::source_fs) nullable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::source_fs) struct ParsedSlotMetadata {
    pub(in crate::source_fs) id: String,
    pub(in crate::source_fs) targets: Vec<String>,
}

#[derive(Debug)]
pub(in crate::source_fs) struct ParsedSqlcompBlock<'a> {
    pub(in crate::source_fs) block: &'a SqlcompBlock,
    pub(in crate::source_fs) annotation: SqlcompAnnotation,
}

pub(in crate::source_fs) fn parse_sqlcomp_annotation(
    block: &SqlcompBlock,
) -> core::DiagnosticResult<SqlcompAnnotation> {
    let annotation_type = parse_annotation_type(block)?;

    match annotation_type.as_str() {
        "query" => parse_sqlcomp_query_metadata(block).map(SqlcompAnnotation::Query),
        "fragment" => parse_fragment_metadata(block).map(SqlcompAnnotation::Fragment),
        "param" => parse_param_metadata(block).map(SqlcompAnnotation::Param),
        "paramEnd" => {
            parse_param_end_metadata(block)?;
            Ok(SqlcompAnnotation::ParamEnd)
        }
        "slot" => parse_slot_metadata(block).map(SqlcompAnnotation::Slot),
        "param_end" => Err(metadata_error(
            "unsupported `@sqlcomp` annotation type `param_end`; use `paramEnd` for Param end markers",
            block.payload_range(),
        )),
        _ => Err(metadata_error(
            format!(
                "unsupported `@sqlcomp` annotation type `{annotation_type}`; supported values are `query`, `fragment`, `param`, `paramEnd`, and `slot`"
            ),
            block.payload_range(),
        )),
    }
}

fn parse_annotation_type(block: &SqlcompBlock) -> core::DiagnosticResult<String> {
    match deserialize_sqlcomp_metadata::<RawSqlcompAnnotationType>(block) {
        Ok(raw) => {
            let Some(annotation_type) = raw.annotation_type else {
                return Err(metadata_error(
                    "missing required `@sqlcomp` metadata field `type`",
                    block.payload_range(),
                ));
            };
            Ok(annotation_type)
        }
        Err(report) => parse_sqlcomp_metadata_object(block)
            .and_then(|metadata| required_annotation_type_from_metadata(&metadata, block))
            .map_err(|_| report),
    }
}

fn parse_query_metadata(
    raw: RawSqlcompQueryMetadata,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<core::QueryMetadata> {
    let Some(id) = raw.id else {
        return Err(metadata_error(
            "missing required `@sqlcomp` metadata field `id`",
            block.payload_range(),
        ));
    };

    if !is_valid_query_id(&id) {
        return Err(metadata_error(
            format!("invalid query id `{id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
            block.payload_range(),
        ));
    }

    Ok(core::QueryMetadata::new(
        id,
        parse_cardinality(raw.cardinality, block)?,
    ))
}

fn parse_fragment_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<core::FragmentMetadata> {
    let metadata = parse_sqlcomp_metadata_object(block)?;
    reject_unknown_metadata_fields(
        &metadata,
        &["type", "id"],
        "fragment",
        "`type` and `id`",
        block,
    )?;
    let id = required_fragment_string_metadata_field(&metadata, "id", block)?;

    if !is_valid_query_id(&id) {
        return Err(metadata_error(
            format!("invalid fragment id `{id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
            block.payload_range(),
        ));
    }

    Ok(core::FragmentMetadata::new(id))
}

fn parse_param_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<ParsedParamMetadata> {
    match parse_sqlcomp_metadata_object(block) {
        Ok(metadata) => parse_param_metadata_object(&metadata, block),
        Err(_) => parse_param_metadata_raw(
            deserialize_sqlcomp_metadata::<RawSqlcompParamMetadata>(block)?,
            block,
        ),
    }
}

fn parse_param_metadata_object(
    metadata: &Map<String, Value>,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<ParsedParamMetadata> {
    reject_unknown_metadata_fields(
        metadata,
        &["type", "id", "valueType", "nullable"],
        "param",
        "`type`, `id`, `valueType`, and `nullable`",
        block,
    )?;
    let id = required_param_string_metadata_field(metadata, "id", block)?;
    if !is_valid_query_id(&id) {
        return Err(metadata_error(
            format!("invalid Param id `{id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
            block.payload_range(),
        ));
    }

    let value_type = parse_param_value_type(
        optional_string_metadata_field(metadata, "valueType", block)?.as_deref(),
        block,
    )?;
    let nullable = if let Some(nullable) = metadata.get("nullable") {
        match nullable {
            Value::Bool(true) => true,
            Value::Bool(false) => {
                return Err(metadata_error(
                    "`nullable: false` is not supported for Param metadata; omit `nullable` for non-null inputs",
                    block.payload_range(),
                ));
            }
            _ => {
                return Err(metadata_error(
                    "`param` metadata field `nullable` must be `true`",
                    block.payload_range(),
                ));
            }
        }
    } else {
        false
    };

    Ok(ParsedParamMetadata {
        id,
        value_type,
        nullable,
    })
}

fn parse_param_metadata_raw(
    raw: RawSqlcompParamMetadata,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<ParsedParamMetadata> {
    let RawSqlcompParamMetadata {
        annotation_type,
        id,
        value_type,
        nullable,
    } = raw;

    if annotation_type.as_deref() != Some("param") {
        return Err(metadata_error(
            "expected `param` metadata",
            block.payload_range(),
        ));
    }

    let Some(id) = id else {
        return Err(metadata_error(
            "missing required `param` metadata field `id`",
            block.payload_range(),
        ));
    };
    if !is_valid_query_id(&id) {
        return Err(metadata_error(
            format!("invalid Param id `{id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
            block.payload_range(),
        ));
    }

    Ok(ParsedParamMetadata {
        id,
        value_type: parse_param_value_type(value_type.as_deref(), block)?,
        nullable: validate_param_nullable(nullable, block)?,
    })
}

fn parse_param_end_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<()> {
    let metadata = parse_sqlcomp_metadata_object(block)?;
    reject_unknown_metadata_fields(&metadata, &["type"], "paramEnd", "`type`", block)
}

fn parse_slot_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<ParsedSlotMetadata> {
    let metadata = parse_sqlcomp_metadata_object(block)?;
    reject_unknown_metadata_fields(
        &metadata,
        &["type", "id", "targets"],
        "slot",
        "`type`, `id`, and `targets`",
        block,
    )?;
    let id = required_slot_string_metadata_field(&metadata, "id", block)?;
    if !is_valid_query_id(&id) {
        return Err(metadata_error(
            format!("invalid Slot id `{id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"),
            block.payload_range(),
        ));
    }
    let targets = required_slot_targets_metadata_field(&metadata, block)?;

    Ok(ParsedSlotMetadata { id, targets })
}

#[derive(Debug, Deserialize)]
struct RawSqlcompAnnotationType {
    #[serde(rename = "type")]
    annotation_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSqlcompQueryMetadata {
    #[serde(rename = "type")]
    annotation_type: Option<String>,
    id: Option<String>,
    cardinality: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawSqlcompParamMetadata {
    #[serde(rename = "type")]
    annotation_type: Option<String>,
    id: Option<String>,
    value_type: Option<String>,
    nullable: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_block() -> SqlcompBlock {
        let position =
            core::SourcePosition::one_based(1, 1).expect("test position should be valid");
        let range = core::SourceRange::point(position);

        SqlcompBlock::new(String::new(), range, range)
    }

    fn raw_param(
        annotation_type: Option<&str>,
        id: Option<&str>,
        value_type: Option<&str>,
        nullable: Option<bool>,
    ) -> RawSqlcompParamMetadata {
        RawSqlcompParamMetadata {
            annotation_type: annotation_type.map(str::to_owned),
            id: id.map(str::to_owned),
            value_type: value_type.map(str::to_owned),
            nullable,
        }
    }

    #[test]
    fn parse_param_metadata_raw_accepts_valid_optional_fields() {
        for (raw, expected_type, expected_nullable) in [
            (
                raw_param(Some("param"), Some("email"), Some("string"), Some(true)),
                Some(core::CoreType::String),
                true,
            ),
            (
                raw_param(Some("param"), Some("tenantId"), None, None),
                None,
                false,
            ),
        ] {
            let metadata =
                parse_param_metadata_raw(raw, &test_block()).expect("raw Param metadata parses");

            assert_eq!(metadata.value_type, expected_type);
            assert_eq!(metadata.nullable, expected_nullable);
        }
    }

    #[test]
    fn parse_param_metadata_raw_rejects_invalid_required_fields() {
        for (raw, expected_message) in [
            (
                raw_param(Some("query"), Some("email"), Some("string"), None),
                "expected `param` metadata",
            ),
            (
                raw_param(Some("param"), None, Some("string"), None),
                "missing required `param` metadata field `id`",
            ),
            (
                raw_param(Some("param"), Some("1bad"), Some("string"), None),
                "invalid Param id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
            ),
            (
                raw_param(Some("param"), Some("email"), Some("string"), Some(false)),
                "`nullable: false` is not supported for Param metadata; omit `nullable` for non-null inputs",
            ),
        ] {
            let report = parse_param_metadata_raw(raw, &test_block())
                .expect_err("invalid raw Param metadata rejected");
            let diagnostic = report
                .diagnostics()
                .first()
                .expect("a diagnostic should be returned");

            assert_eq!(diagnostic.message(), expected_message);
        }
    }
}
