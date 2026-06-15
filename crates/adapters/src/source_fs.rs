//! Filesystem source intake adapter.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use sqlcomp_app::{SourceRead, SourceReader};
use sqlcomp_core as core;

const SQLCOMP_MARKER: &str = "@sqlcomp";
const SUPPORTED_PARAM_VALUE_TYPES: [&str; 11] = [
    "bool", "int32", "int64", "float64", "decimal", "string", "bytes", "date", "time", "datetime",
    "json",
];

/// Result of scanning SQL text for sqlcomp metadata blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlcompBlockScan {
    blocks: Vec<SqlcompBlock>,
    sql_without_sqlcomp_blocks: String,
}

impl SqlcompBlockScan {
    /// Build a scan result.
    #[must_use]
    pub const fn new(blocks: Vec<SqlcompBlock>, sql_without_sqlcomp_blocks: String) -> Self {
        Self {
            blocks,
            sql_without_sqlcomp_blocks,
        }
    }

    /// Metadata blocks found in source order.
    #[must_use]
    pub fn blocks(&self) -> &[SqlcompBlock] {
        &self.blocks
    }

    /// SQL text with sqlcomp metadata comments replaced by whitespace.
    #[must_use]
    pub fn sql_without_sqlcomp_blocks(&self) -> &str {
        &self.sql_without_sqlcomp_blocks
    }
}

/// One `/* @sqlcomp ... */` metadata block.
#[derive(Clone, Debug)]
pub struct SqlcompBlock {
    payload: String,
    comment_range: core::SourceRange,
    payload_range: core::SourceRange,
    comment_start_index: usize,
    comment_end_index: usize,
}

impl SqlcompBlock {
    /// Build a sqlcomp metadata block.
    #[must_use]
    pub const fn new(
        payload: String,
        comment_range: core::SourceRange,
        payload_range: core::SourceRange,
    ) -> Self {
        Self::from_scan(payload, comment_range, payload_range, 0, 0)
    }

    const fn from_scan(
        payload: String,
        comment_range: core::SourceRange,
        payload_range: core::SourceRange,
        comment_start_index: usize,
        comment_end_index: usize,
    ) -> Self {
        Self {
            payload,
            comment_range,
            payload_range,
            comment_start_index,
            comment_end_index,
        }
    }

    /// Raw metadata payload after the `@sqlcomp` marker.
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    /// Source range for the full block comment.
    #[must_use]
    pub const fn comment_range(&self) -> core::SourceRange {
        self.comment_range
    }

    /// Source range for the metadata payload.
    #[must_use]
    pub const fn payload_range(&self) -> core::SourceRange {
        self.payload_range
    }

    const fn comment_start_index(&self) -> usize {
        self.comment_start_index
    }

    const fn comment_end_index(&self) -> usize {
        self.comment_end_index
    }
}

impl PartialEq for SqlcompBlock {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
            && self.comment_range == other.comment_range
            && self.payload_range == other.payload_range
    }
}

impl Eq for SqlcompBlock {}

/// Scan SQL source for canonical `@sqlcomp` block comments.
///
/// # Errors
///
/// Returns a diagnostic when a SQL block comment is not terminated.
pub fn scan_sqlcomp_blocks(source: &str) -> core::DiagnosticResult<SqlcompBlockScan> {
    Scanner::new(source).scan()
}

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
enum SqlcompAnnotation {
    Query(core::QueryMetadata),
    Param,
    ParamEnd,
}

#[derive(Debug)]
struct ParsedSqlcompBlock<'a> {
    block: &'a SqlcompBlock,
    annotation: SqlcompAnnotation,
}

fn parse_sqlcomp_annotation(block: &SqlcompBlock) -> core::DiagnosticResult<SqlcompAnnotation> {
    let annotation_type = parse_annotation_type(block)?;

    match annotation_type.as_str() {
        "query" => parse_sqlcomp_query_metadata(block).map(SqlcompAnnotation::Query),
        "param" => {
            parse_param_metadata(block)?;
            Ok(SqlcompAnnotation::Param)
        }
        "paramEnd" => {
            parse_param_end_metadata(block)?;
            Ok(SqlcompAnnotation::ParamEnd)
        }
        "param_end" => Err(metadata_error(
            "unsupported `@sqlcomp` annotation type `param_end`; use `paramEnd` for Param end markers",
            block.payload_range(),
        )),
        _ => Err(metadata_error(
            format!(
                "unsupported `@sqlcomp` annotation type `{annotation_type}`; supported values are `query`, `param`, and `paramEnd`"
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

fn parse_sqlcomp_metadata_object(
    block: &SqlcompBlock,
) -> core::DiagnosticResult<Map<String, Value>> {
    let value = parse_sqlcomp_metadata_value(block)?;
    let Value::Object(metadata) = value else {
        return Err(metadata_error(
            "`@sqlcomp` metadata must be an object",
            block.payload_range(),
        ));
    };

    Ok(metadata)
}

fn parse_sqlcomp_metadata_value(block: &SqlcompBlock) -> core::DiagnosticResult<Value> {
    match deserialize_sqlcomp_metadata(block) {
        Ok(value) => Ok(value),
        Err(report) => parse_flat_sqlcomp_metadata_value(block.payload()).ok_or(report),
    }
}

fn deserialize_sqlcomp_metadata<T>(block: &SqlcompBlock) -> core::DiagnosticResult<T>
where
    T: DeserializeOwned,
{
    match deser_hjson::from_str::<T>(block.payload()) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(normalized) = normalize_inline_hjson_metadata(block.payload())
                && let Ok(value) = deser_hjson::from_str::<T>(&normalized)
            {
                return Ok(value);
            }

            Err(metadata_error(
                format!("failed to parse `@sqlcomp` metadata as Hjson: {error}"),
                block.payload_range(),
            ))
        }
    }
}

fn normalize_inline_hjson_metadata(payload: &str) -> Option<String> {
    let mut normalized = String::with_capacity(payload.len());
    let mut index = 0;
    let mut changed = false;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while index < payload.len() {
        let char = payload[index..]
            .chars()
            .next()
            .expect("index should point at a character");

        if !in_single_quote && !in_double_quote && char.is_whitespace() {
            let whitespace_start = index;
            while index < payload.len() {
                let whitespace_char = payload[index..]
                    .chars()
                    .next()
                    .expect("index should point at a character");
                if !whitespace_char.is_whitespace() {
                    break;
                }
                index += whitespace_char.len_utf8();
            }

            let previous_significant = last_non_whitespace_char(&normalized);
            let should_insert_line_break = metadata_key_starts(&payload[index..])
                && !matches!(previous_significant, None | Some(',' | '\n' | '\r'))
                || payload[index..].starts_with('}')
                    && !matches!(previous_significant, None | Some('{' | ',' | '\n' | '\r'));
            if should_insert_line_break {
                normalized.push('\n');
                changed = true;
            } else {
                normalized.push_str(&payload[whitespace_start..index]);
            }
            continue;
        }

        if !in_double_quote && char == '\'' {
            in_single_quote = !in_single_quote;
        } else if !in_single_quote && char == '"' {
            in_double_quote = !in_double_quote;
        }

        normalized.push(char);
        index += char.len_utf8();
    }

    changed.then_some(normalized)
}

fn metadata_key_starts(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    for char in chars {
        if char == ':' {
            return true;
        }
        if !(char == '_' || char.is_ascii_alphanumeric()) {
            return false;
        }
    }

    false
}

fn parse_flat_sqlcomp_metadata_value(payload: &str) -> Option<Value> {
    let normalized = normalize_inline_hjson_metadata(payload)?;
    let trimmed = normalized.trim();
    let inner = trimmed.strip_prefix('{')?.strip_suffix('}')?;
    let mut metadata = Map::new();

    for line in inner.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() {
            continue;
        }

        let (key, value) = line.split_once(':')?;
        let key = key.trim();
        if !is_metadata_key(key) {
            return None;
        }

        metadata.insert(key.to_owned(), flat_metadata_value(value.trim())?);
    }

    Some(Value::Object(metadata))
}

fn is_metadata_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|char| char == '_' || char.is_ascii_alphanumeric())
}

fn flat_metadata_value(value: &str) -> Option<Value> {
    if value.is_empty() {
        return None;
    }

    match value {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => {
            let unquoted = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|value| value.strip_suffix('\''))
                })
                .unwrap_or(value);
            Some(Value::String(unquoted.to_owned()))
        }
    }
}

fn last_non_whitespace_char(source: &str) -> Option<char> {
    source.chars().rev().find(|char| !char.is_whitespace())
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

fn parse_param_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<()> {
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
) -> core::DiagnosticResult<()> {
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

    let value_type = optional_string_metadata_field(metadata, "valueType", block)?;
    validate_param_value_type(value_type.as_deref(), block)?;
    if let Some(nullable) = metadata.get("nullable") {
        match nullable {
            Value::Bool(true) => {}
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
    }

    Ok(())
}

fn parse_param_metadata_raw(
    raw: RawSqlcompParamMetadata,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<()> {
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

    validate_param_value_type(value_type.as_deref(), block)?;
    validate_param_nullable(nullable, block)
}

fn parse_param_end_metadata(block: &SqlcompBlock) -> core::DiagnosticResult<()> {
    let metadata = parse_sqlcomp_metadata_object(block)?;
    reject_unknown_metadata_fields(&metadata, &["type"], "paramEnd", "`type`", block)
}

fn reject_unknown_metadata_fields(
    metadata: &Map<String, Value>,
    allowed_fields: &[&str],
    annotation_type: &str,
    supported_fields: &str,
    block: &SqlcompBlock,
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

fn required_annotation_type_from_metadata(
    metadata: &Map<String, Value>,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<String> {
    match metadata.get("type") {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(metadata_error(
            "`@sqlcomp` metadata field `type` must be a string",
            block.payload_range(),
        )),
        None => Err(metadata_error(
            "missing required `@sqlcomp` metadata field `type`",
            block.payload_range(),
        )),
    }
}

fn required_param_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<String> {
    match metadata.get(field) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(metadata_error(
            format!("`param` metadata field `{field}` must be a string"),
            block.payload_range(),
        )),
        None => Err(metadata_error(
            format!("missing required `param` metadata field `{field}`"),
            block.payload_range(),
        )),
    }
}

fn optional_string_metadata_field(
    metadata: &Map<String, Value>,
    field: &str,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<Option<String>> {
    match metadata.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(metadata_error(
            format!("`@sqlcomp` metadata field `{field}` must be a string"),
            block.payload_range(),
        )),
        None => Ok(None),
    }
}

fn validate_param_nullable(
    nullable: Option<bool>,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<()> {
    match nullable {
        Some(true) | None => Ok(()),
        Some(false) => Err(metadata_error(
            "`nullable: false` is not supported for Param metadata; omit `nullable` for non-null inputs",
            block.payload_range(),
        )),
    }
}

fn validate_param_value_type(
    value_type: Option<&str>,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<()> {
    let Some(value_type) = value_type else {
        return Ok(());
    };

    if value_type.is_empty() {
        return Err(metadata_error(
            "`param` metadata field `valueType` must not be empty",
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

    Ok(())
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

fn parse_cardinality(
    raw_cardinality: Option<String>,
    block: &SqlcompBlock,
) -> core::DiagnosticResult<Option<core::Cardinality>> {
    let Some(raw_cardinality) = raw_cardinality else {
        return Ok(None);
    };

    match raw_cardinality.as_str() {
        "one" => Ok(Some(core::Cardinality::One)),
        "many" => Ok(Some(core::Cardinality::Many)),
        "exec" => Err(metadata_error(
            "`cardinality: exec` is reserved for future non-SELECT support and is not supported in the MVP",
            block.payload_range(),
        )),
        _ => Err(metadata_error(
            format!(
                "unsupported query cardinality `{raw_cardinality}`; supported MVP values are `one` and `many`"
            ),
            block.payload_range(),
        )),
    }
}

/// Split SQL source text into raw query blocks.
///
/// # Errors
///
/// Returns diagnostics when sqlcomp block scanning fails or any query metadata
/// payload is invalid.
pub fn split_sqlcomp_query_blocks(source: &str) -> core::DiagnosticResult<Vec<core::RawQuery>> {
    let scan = scan_sqlcomp_blocks(source)?;
    split_sqlcomp_query_blocks_from_scan(source, &scan)
}

fn split_sqlcomp_query_blocks_from_scan(
    source: &str,
    scan: &SqlcompBlockScan,
) -> core::DiagnosticResult<Vec<core::RawQuery>> {
    let blocks = scan.blocks();
    let mut parsed_blocks = Vec::with_capacity(blocks.len());

    for block in blocks {
        parsed_blocks.push(ParsedSqlcompBlock {
            block,
            annotation: parse_sqlcomp_annotation(block)?,
        });
    }

    validate_inline_param_markers(&parsed_blocks)?;

    let query_indexes = parsed_blocks
        .iter()
        .enumerate()
        .filter_map(|(index, parsed_block)| {
            matches!(parsed_block.annotation, SqlcompAnnotation::Query(_)).then_some(index)
        })
        .collect::<Vec<_>>();
    let mut queries = Vec::with_capacity(query_indexes.len());

    for (query_position, parsed_index) in query_indexes.iter().copied().enumerate() {
        let parsed_block = &parsed_blocks[parsed_index];
        let SqlcompAnnotation::Query(metadata) = &parsed_block.annotation else {
            unreachable!("query indexes only point at query annotations");
        };
        let body_start = parsed_block.block.comment_end_index();
        let body_end = query_indexes
            .get(query_position + 1)
            .map_or(source.len(), |next_query_index| {
                parsed_blocks[*next_query_index].block.comment_start_index()
            });
        let sql = source[body_start..body_end].to_owned();
        let location = core::SourceLocation::from_range(source_range_for_sql_body(
            source, body_start, body_end,
        ));

        queries.push(core::RawQuery::new(metadata.clone(), sql).with_source_location(location));
    }

    Ok(queries)
}

fn validate_inline_param_markers(
    parsed_blocks: &[ParsedSqlcompBlock<'_>],
) -> core::DiagnosticResult<()> {
    let mut inside_query = false;
    let mut open_param_block: Option<&SqlcompBlock> = None;

    for parsed_block in parsed_blocks {
        match parsed_block.annotation {
            SqlcompAnnotation::Query(_) => {
                if let Some(block) = open_param_block.take() {
                    return Err(metadata_error(
                        "`param` marker is missing a matching `paramEnd` marker",
                        block.payload_range(),
                    ));
                }
                inside_query = true;
            }
            SqlcompAnnotation::Param => {
                if !inside_query {
                    return Err(metadata_error(
                        "Param markers must appear inside a query body",
                        parsed_block.block.payload_range(),
                    ));
                }
                if open_param_block.is_some() {
                    return Err(metadata_error(
                        "nested Param ranges are not supported",
                        parsed_block.block.payload_range(),
                    ));
                }
                open_param_block = Some(parsed_block.block);
            }
            SqlcompAnnotation::ParamEnd => {
                if !inside_query {
                    return Err(metadata_error(
                        "Param markers must appear inside a query body",
                        parsed_block.block.payload_range(),
                    ));
                }
                if open_param_block.take().is_none() {
                    return Err(metadata_error(
                        "`paramEnd` marker has no matching `param` marker",
                        parsed_block.block.payload_range(),
                    ));
                }
            }
        }
    }

    if let Some(block) = open_param_block {
        return Err(metadata_error(
            "`param` marker is missing a matching `paramEnd` marker",
            block.payload_range(),
        ));
    }

    Ok(())
}

/// Dummy filesystem-backed source reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemSourceReader;

impl SourceReader for FileSystemSourceReader {
    fn read(&self, plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        let mut seen_ids = HashMap::new();
        let mut queries = Vec::new();
        let mut diagnostics = core::DiagnosticReport::default();
        let mut fatal_diagnostics = core::DiagnosticReport::default();

        for path in discover_source_files(plan)? {
            let Some(source_path) = plan.source_relative_path(&path) else {
                extend_diagnostics(
                    &mut fatal_diagnostics,
                    file_error(
                        format!(
                            "source file `{}` is outside the configuration directory `{}`",
                            path.display(),
                            plan.config_dir().display()
                        ),
                        &path,
                    ),
                );
                continue;
            };
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    extend_diagnostics(
                        &mut fatal_diagnostics,
                        file_error(
                            format!(
                                "failed to read SQL source file `{}`: {error}",
                                path.display()
                            ),
                            &path,
                        ),
                    );
                    continue;
                }
            };
            let scan = match scan_sqlcomp_blocks(&source) {
                Ok(scan) => scan,
                Err(report) => {
                    extend_diagnostics(&mut fatal_diagnostics, attach_path(report, &path));
                    continue;
                }
            };
            if scan.blocks().is_empty()
                && contains_non_comment_sql(scan.sql_without_sqlcomp_blocks())
            {
                diagnostics.push(unannotated_sql_warning(&path));
            }

            let file_queries = match split_sqlcomp_query_blocks_from_scan(&source, &scan) {
                Ok(file_queries) => file_queries,
                Err(report) => {
                    extend_diagnostics(&mut fatal_diagnostics, attach_path(report, &path));
                    continue;
                }
            };
            let file_queries = file_queries
                .into_iter()
                .map(|query| attach_query_path(query, &path).with_source_path(source_path.clone()))
                .collect::<Vec<_>>();
            collect_duplicate_query_ids(&file_queries, &mut seen_ids, &mut fatal_diagnostics);
            queries.extend(file_queries);
        }

        if !fatal_diagnostics.is_empty() {
            return Err(fatal_diagnostics);
        }

        Ok(SourceRead::new(queries, diagnostics))
    }
}

fn extend_diagnostics(diagnostics: &mut core::DiagnosticReport, report: core::DiagnosticReport) {
    for diagnostic in report.into_diagnostics() {
        diagnostics.push(diagnostic);
    }
}

fn attach_query_path(query: core::RawQuery, path: &Path) -> core::RawQuery {
    let range = query
        .source_location()
        .and_then(core::SourceLocation::range);

    if let Some(range) = range {
        query.with_source_location(core::SourceLocation::at_range(path, range))
    } else {
        query.with_source_location(core::SourceLocation::for_path(path))
    }
}

fn discover_source_files(plan: &core::CompilationPlan) -> core::DiagnosticResult<Vec<PathBuf>> {
    let mut files = BTreeSet::new();

    for include in plan.source_include() {
        for path in files_matching_pattern(include)? {
            if is_sql_file(&path) && !is_excluded(&path, plan.source_exclude()) {
                files.insert(path);
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn files_matching_pattern(pattern: &Path) -> core::DiagnosticResult<Vec<PathBuf>> {
    if !path_has_glob(pattern) {
        return Ok(pattern
            .is_file()
            .then(|| pattern.to_path_buf())
            .into_iter()
            .collect());
    }

    let root = static_glob_prefix(pattern);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_matching_files(&root, pattern, &mut files)?;
    Ok(files)
}

fn collect_matching_files(
    directory: &Path,
    pattern: &Path,
    files: &mut Vec<PathBuf>,
) -> core::DiagnosticResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| {
            file_error(
                format!(
                    "failed to read source directory `{}`: {error}",
                    directory.display()
                ),
                directory,
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            file_error(
                format!(
                    "failed to read an entry in source directory `{}`: {error}",
                    directory.display()
                ),
                directory,
            )
        })?;

    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            file_error(
                format!(
                    "failed to inspect source path `{}`: {error}",
                    path.display()
                ),
                &path,
            )
        })?;

        if file_type.is_dir() {
            collect_matching_files(&path, pattern, files)?;
        } else if file_type.is_file() && path_matches_pattern(&path, pattern) {
            files.push(path);
        }
    }

    Ok(())
}

fn static_glob_prefix(pattern: &Path) -> PathBuf {
    let mut prefix = PathBuf::new();

    for component in pattern.components() {
        if component_has_glob(component) {
            break;
        }
        prefix.push(component.as_os_str());
    }

    if prefix.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        prefix
    }
}

fn path_matches_pattern(path: &Path, pattern: &Path) -> bool {
    let path_components = normalized_path_components(path);
    let pattern_components = normalized_path_components(pattern);

    path_components_match(&pattern_components, &path_components)
}

fn path_components_match(pattern: &[String], path: &[String]) -> bool {
    match (pattern.split_first(), path.split_first()) {
        (None, None) => true,
        (Some((component, remaining_pattern)), _) if component == "**" => {
            path_components_match(remaining_pattern, path)
                || path.split_first().is_some_and(|(_, remaining_path)| {
                    path_components_match(pattern, remaining_path)
                })
        }
        (Some((component, remaining_pattern)), Some((path_component, remaining_path))) => {
            component_matches_pattern(component, path_component)
                && path_components_match(remaining_pattern, remaining_path)
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn component_matches_pattern(pattern: &str, value: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let value = value.chars().collect::<Vec<_>>();

    component_chars_match(&pattern, &value)
}

fn component_chars_match(pattern: &[char], value: &[char]) -> bool {
    match (pattern.split_first(), value.split_first()) {
        (None, None) => true,
        (Some(('*', remaining_pattern)), _) => {
            component_chars_match(remaining_pattern, value)
                || value.split_first().is_some_and(|(_, remaining_value)| {
                    component_chars_match(pattern, remaining_value)
                })
        }
        (Some(('?', remaining_pattern)), Some((_, remaining_value))) => {
            component_chars_match(remaining_pattern, remaining_value)
        }
        (Some((pattern_char, remaining_pattern)), Some((value_char, remaining_value))) => {
            pattern_char == value_char && component_chars_match(remaining_pattern, remaining_value)
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn normalized_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir => Some(String::new()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_owned()),
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
        })
        .collect()
}

fn is_excluded(path: &Path, exclude_patterns: &[PathBuf]) -> bool {
    exclude_patterns
        .iter()
        .any(|pattern| path_matches_pattern(path, pattern))
}

fn is_sql_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "sql")
}

fn path_has_glob(path: &Path) -> bool {
    path.components().any(component_has_glob)
}

fn component_has_glob(component: Component<'_>) -> bool {
    component
        .as_os_str()
        .to_string_lossy()
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?'))
}

fn file_error(message: impl Into<String>, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::for_path(path)),
    )
}

fn attach_path(report: core::DiagnosticReport, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::from_diagnostics(
        report
            .into_diagnostics()
            .into_iter()
            .map(|diagnostic| {
                if diagnostic
                    .location()
                    .and_then(core::SourceLocation::path)
                    .is_some()
                {
                    return diagnostic;
                }

                let location = diagnostic
                    .location()
                    .and_then(core::SourceLocation::range)
                    .map_or_else(
                        || core::SourceLocation::for_path(path),
                        |range| core::SourceLocation::at_range(path, range),
                    );

                core::Diagnostic::new(diagnostic.severity(), diagnostic.message())
                    .with_location(location)
            })
            .collect(),
    )
}

fn unannotated_sql_warning(path: &Path) -> core::Diagnostic {
    core::Diagnostic::warning(
        "included SQL file contains SQL but no `@sqlcomp` query annotation; add a `/* @sqlcomp { type: query, id: ... } */` block before the query",
    )
    .with_location(core::SourceLocation::for_path(path))
}

fn contains_non_comment_sql(source: &str) -> bool {
    NonCommentSqlScanner::new(source).contains_sql()
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueryDeclaration {
    location: Option<core::SourceLocation>,
}

type SeenQueryIds = HashMap<String, QueryDeclaration>;

fn collect_duplicate_query_ids(
    queries: &[core::RawQuery],
    seen_ids: &mut SeenQueryIds,
    diagnostics: &mut core::DiagnosticReport,
) {
    for query in queries {
        let declaration = QueryDeclaration {
            location: query.source_location().cloned(),
        };

        if let Some(first_declaration) = seen_ids.get(query.metadata().id()) {
            diagnostics.push(
                core::Diagnostic::error(format!(
                    "duplicate query id `{}`; query IDs must be unique across the full compile run",
                    query.metadata().id()
                ))
                .with_location(
                    query
                        .source_location()
                        .cloned()
                        .unwrap_or_else(core::SourceLocation::unknown),
                ),
            );
            diagnostics.push(
                core::Diagnostic::note("first declared here").with_location(
                    first_declaration
                        .location
                        .clone()
                        .unwrap_or_else(core::SourceLocation::unknown),
                ),
            );
        } else {
            seen_ids.insert(query.metadata().id().to_owned(), declaration);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TextPosition {
    line: usize,
    column: usize,
}

impl TextPosition {
    const START: Self = Self { line: 1, column: 1 };

    const fn advance(&mut self, char: char) {
        if char == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    fn into_source_position(self) -> core::SourcePosition {
        core::SourcePosition::one_based(self.line, self.column)
            .expect("scanner positions are always one-based")
    }
}

struct Scanner<'a> {
    source: &'a str,
    index: usize,
    position: TextPosition,
    blocks: Vec<SqlcompBlock>,
    sql_without_sqlcomp_blocks: String,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            index: 0,
            position: TextPosition::START,
            blocks: Vec::new(),
            sql_without_sqlcomp_blocks: String::with_capacity(source.len()),
        }
    }

    fn scan(mut self) -> core::DiagnosticResult<SqlcompBlockScan> {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.scan_block_comment()?;
            } else if self.is_line_comment_start() {
                self.copy_line_comment();
            } else if self.current_char().is_some_and(is_quote_delimiter) {
                self.copy_quoted();
            } else {
                self.copy_current();
            }
        }

        Ok(SqlcompBlockScan::new(
            self.blocks,
            self.sql_without_sqlcomp_blocks,
        ))
    }

    fn scan_block_comment(&mut self) -> core::DiagnosticResult<()> {
        let comment_start_index = self.index;
        let comment_start = self.position;
        self.advance_current();
        self.advance_current();
        let body_start_index = self.index;

        while !self.is_at_end() {
            if self.starts_with("*/") {
                let body_end_index = self.index;
                self.advance_current();
                self.advance_current();
                let comment_end_index = self.index;

                self.push_scanned_comment(
                    comment_start_index,
                    body_start_index,
                    body_end_index,
                    comment_end_index,
                );
                return Ok(());
            }

            self.advance_current();
        }

        Err(core::DiagnosticReport::new(
            core::Diagnostic::error("unterminated SQL block comment").with_location(
                core::SourceLocation::from_range(core::SourceRange::point(
                    comment_start.into_source_position(),
                )),
            ),
        ))
    }

    fn push_scanned_comment(
        &mut self,
        comment_start_index: usize,
        body_start_index: usize,
        body_end_index: usize,
        comment_end_index: usize,
    ) {
        let body = &self.source[body_start_index..body_end_index];
        if let Some(marker_offset) = sqlcomp_marker_offset(body) {
            let payload_start_index = body_start_index + marker_offset + SQLCOMP_MARKER.len();
            let payload = self.source[payload_start_index..body_end_index].to_owned();
            let comment_range =
                source_range_for_span(self.source, comment_start_index, comment_end_index);
            let payload_range =
                source_range_for_span(self.source, payload_start_index, body_end_index);

            self.blocks.push(SqlcompBlock::from_scan(
                payload,
                comment_range,
                payload_range,
                comment_start_index,
                comment_end_index,
            ));
            self.sql_without_sqlcomp_blocks.push_str(&blank_comment(
                &self.source[comment_start_index..comment_end_index],
            ));
        } else {
            self.sql_without_sqlcomp_blocks
                .push_str(&self.source[comment_start_index..comment_end_index]);
        }
    }

    fn copy_quoted(&mut self) {
        let delimiter = self
            .current_char()
            .expect("quoted copy should start at a delimiter");
        self.copy_current();

        while let Some(char) = self.current_char() {
            self.copy_current();

            if delimiter != '`' && char == '\\' {
                if !self.is_at_end() {
                    self.copy_current();
                }
                continue;
            }

            if char == delimiter {
                if self.current_char() == Some(delimiter) {
                    self.copy_current();
                } else {
                    break;
                }
            }
        }
    }

    fn copy_line_comment(&mut self) {
        while let Some(char) = self.current_char() {
            self.copy_current();
            if char == '\n' {
                break;
            }
        }
    }

    fn copy_current(&mut self) {
        let char = self
            .advance_current()
            .expect("copy_current should only be called before EOF");
        self.sql_without_sqlcomp_blocks.push(char);
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        self.position.advance(char);
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.source.len()
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}

struct NonCommentSqlScanner<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> NonCommentSqlScanner<'a> {
    const fn new(source: &'a str) -> Self {
        Self { source, index: 0 }
    }

    fn contains_sql(mut self) -> bool {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(char::is_whitespace) {
                self.advance_current();
            } else {
                return true;
            }
        }

        false
    }

    fn skip_block_comment(&mut self) {
        self.advance_current();
        self.advance_current();

        while !self.is_at_end() {
            if self.starts_with("*/") {
                self.advance_current();
                self.advance_current();
                return;
            }

            self.advance_current();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(char) = self.advance_current() {
            if char == '\n' {
                return;
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.source.len()
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}

const fn is_quote_delimiter(char: char) -> bool {
    matches!(char, '\'' | '"' | '`')
}

fn sqlcomp_marker_offset(body: &str) -> Option<usize> {
    let trimmed = body.trim_start();
    let offset = body.len() - trimmed.len();
    let after_marker = trimmed.strip_prefix(SQLCOMP_MARKER)?;
    let marker_has_boundary = after_marker
        .chars()
        .next()
        .is_none_or(|char| !(char.is_ascii_alphanumeric() || char == '_'));

    marker_has_boundary.then_some(offset)
}

fn blank_comment(comment: &str) -> String {
    comment
        .chars()
        .map(|char| {
            if matches!(char, '\n' | '\r') {
                char
            } else {
                ' '
            }
        })
        .collect()
}

fn source_range_for_span(source: &str, start: usize, end: usize) -> core::SourceRange {
    core::SourceRange::new(
        source_position_at_byte(source, start),
        Some(source_position_at_byte(source, end)),
    )
}

fn source_range_for_sql_body(source: &str, start: usize, end: usize) -> core::SourceRange {
    let sql = &source[start..end];

    if sql.trim().is_empty() {
        return source_range_for_span(source, start, end);
    }

    let trimmed_start = start + sql.len() - sql.trim_start().len();
    let trimmed_end = start + sql.trim_end().len();

    source_range_for_span(source, trimmed_start, trimmed_end)
}

fn source_position_at_byte(source: &str, target: usize) -> core::SourcePosition {
    debug_assert!(source.is_char_boundary(target));

    let mut position = TextPosition::START;
    for (index, char) in source.char_indices() {
        if index >= target {
            break;
        }
        position.advance(char);
    }

    position.into_source_position()
}

fn metadata_error(message: impl Into<String>, range: core::SourceRange) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::from_range(range)),
    )
}

fn is_valid_query_id(id: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{
        FileSystemSourceReader, SqlcompBlock, parse_sqlcomp_query_metadata, scan_sqlcomp_blocks,
        split_sqlcomp_query_blocks,
    };
    use crate::dialect_mysql::MysqlDialectAnalyzer;
    use sqlcomp_app::{DialectAnalyzer, SourceReader};
    use sqlcomp_core as core;

    #[test]
    fn returns_empty_scan_when_no_annotation_exists() {
        let source = "SELECT 'plain sql' AS value;\n";
        let scan = scan_sqlcomp_blocks(source).expect("plain SQL should scan");

        assert!(scan.blocks().is_empty());
        assert_eq!(scan.sql_without_sqlcomp_blocks(), source);
    }

    #[test]
    fn finds_one_sqlcomp_block_and_preserves_sql() {
        let source = r"
/* @sqlcomp
{ type: query, id: listUsers }
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");

        assert_eq!(scan.blocks().len(), 1);
        assert_eq!(
            scan.blocks()[0].payload(),
            "\n{ type: query, id: listUsers }\n"
        );
        assert_eq!(scan.blocks()[0].comment_range().start().line(), 1);
        assert_eq!(scan.blocks()[0].comment_range().start().column(), 1);
        assert_eq!(scan.blocks()[0].payload_range().start().line(), 1);
        assert_eq!(scan.blocks()[0].payload_range().start().column(), 12);
        assert!(!scan.sql_without_sqlcomp_blocks().contains("@sqlcomp"));
        assert!(
            scan.sql_without_sqlcomp_blocks()
                .ends_with("SELECT id FROM users;\n")
        );
    }

    #[test]
    fn scanned_block_equality_ignores_internal_byte_offsets() {
        let source = r"

/* @sqlcomp
{ type: query, id: listUsers }
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let scanned = &scan.blocks()[0];
        let constructed = SqlcompBlock::new(
            scanned.payload().to_owned(),
            scanned.comment_range(),
            scanned.payload_range(),
        );

        assert_eq!(*scanned, constructed);
    }

    #[test]
    fn finds_multiple_sqlcomp_blocks() {
        let source = r"
/* @sqlcomp
{ id: first }
*/
SELECT 1;
/* @sqlcomp
{ id: second }
*/
SELECT 2;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("multiple annotations should scan");

        assert_eq!(scan.blocks().len(), 2);
        assert_eq!(scan.blocks()[0].payload(), "\n{ id: first }\n");
        assert_eq!(scan.blocks()[1].payload(), "\n{ id: second }\n");
        assert_eq!(
            scan.sql_without_sqlcomp_blocks().matches("SELECT").count(),
            2
        );
    }

    #[test]
    fn parses_query_metadata_from_hjson_payload() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
  cardinality: one
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let metadata =
            parse_sqlcomp_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

        assert_eq!(metadata.id(), "listUsers");
        assert_eq!(metadata.cardinality(), Some(core::Cardinality::One));
    }

    #[test]
    fn parses_query_metadata_without_optional_cardinality() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let metadata =
            parse_sqlcomp_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

        assert_eq!(metadata.id(), "listUsers");
        assert_eq!(metadata.cardinality(), None);
    }

    #[test]
    fn accepts_supported_cardinality_values() {
        for (raw_cardinality, cardinality) in [
            ("one", core::Cardinality::One),
            ("many", core::Cardinality::Many),
        ] {
            let source = format!(
                r"
/* @sqlcomp
{{
  type: query
  id: listUsers
  cardinality: {raw_cardinality}
}}
*/
SELECT id FROM users;
"
            );
            let source = source
                .strip_prefix('\n')
                .expect("raw SQL test source should start with a newline");
            let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
            let metadata = parse_sqlcomp_query_metadata(&scan.blocks()[0])
                .expect("query metadata should parse");

            assert_eq!(metadata.cardinality(), Some(cardinality));
        }
    }

    #[test]
    fn rejects_missing_required_query_metadata_fields() {
        for (source, expected_message) in [
            (
                r"
/* @sqlcomp
{
  id: listUsers
}
*/
SELECT id FROM users;
",
                "missing required `@sqlcomp` metadata field `type`",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
}
*/
SELECT id FROM users;
",
                "missing required `@sqlcomp` metadata field `id`",
            ),
        ] {
            let source = source
                .strip_prefix('\n')
                .expect("raw SQL test source should start with a newline");
            let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
            let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
                .expect_err("missing required metadata should be rejected");
            let diagnostic = report
                .diagnostics()
                .first()
                .expect("a diagnostic should be returned");

            assert_eq!(diagnostic.message(), expected_message);
            assert!(diagnostic.location().is_some());
        }
    }

    #[test]
    fn rejects_exec_cardinality_reserved_for_future_mvp_work() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
  cardinality: exec
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
            .expect_err("exec cardinality should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");

        assert_eq!(
            diagnostic.message(),
            "`cardinality: exec` is reserved for future non-SELECT support and is not supported in the MVP"
        );
        assert!(diagnostic.location().is_some());
    }

    #[test]
    fn rejects_unsupported_cardinality_values() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
  cardinality: maybe
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
            .expect_err("unsupported cardinality should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");

        assert_eq!(
            diagnostic.message(),
            "unsupported query cardinality `maybe`; supported MVP values are `one` and `many`"
        );
        assert!(diagnostic.location().is_some());
    }

    #[test]
    fn splits_one_query_block() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source).expect("query block should split");

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].metadata().id(), "listUsers");
        assert_eq!(queries[0].sql(), "\nSELECT id FROM users;\n");
        assert!(!queries[0].sql().contains("@sqlcomp"));
    }

    #[test]
    fn split_query_blocks_attach_sql_body_source_range() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source).expect("query block should split");
        let location = queries[0]
            .source_location()
            .expect("query should include source location");
        let range = location
            .range()
            .expect("query should include SQL body range");

        assert_eq!(location.path(), None);
        assert_eq!(range.start().line(), 7);
        assert_eq!(range.start().column(), 1);
    }

    #[test]
    fn splits_multiple_query_blocks_in_source_order() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: firstQuery
}
*/
SELECT 1;
/* @sqlcomp
{
  type: query
  id: secondQuery
}
*/
SELECT 2;
-- trailing file content
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source).expect("query blocks should split");

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].metadata().id(), "firstQuery");
        assert_eq!(queries[0].sql(), "\nSELECT 1;\n");
        assert_eq!(queries[1].metadata().id(), "secondQuery");
        assert_eq!(queries[1].sql(), "\nSELECT 2;\n-- trailing file content\n");
    }

    #[test]
    fn splits_adjacent_query_blocks() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: firstQuery
}
*/SELECT 1;/* @sqlcomp
{
  type: query
  id: secondQuery
}
*/SELECT 2;"
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source).expect("adjacent queries should split");

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].metadata().id(), "firstQuery");
        assert_eq!(queries[0].sql(), "SELECT 1;");
        assert_eq!(queries[1].metadata().id(), "secondQuery");
        assert_eq!(queries[1].sql(), "SELECT 2;");
    }

    #[test]
    fn split_query_blocks_keeps_inline_param_markers_inside_query_body() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: string nullable: true } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source).expect("inline Param should be accepted");

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].metadata().id(), "findUserByEmail");
        assert!(
            queries[0].sql().contains("type: param id: email"),
            "sql: {}",
            queries[0].sql()
        );
        assert!(
            queries[0].sql().contains("type: paramEnd"),
            "sql: {}",
            queries[0].sql()
        );
    }

    #[test]
    fn split_query_blocks_keeps_multiple_query_boundaries_with_inline_params() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;

/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let queries = split_sqlcomp_query_blocks(source)
            .expect("inline Param should not create extra query boundaries");

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].metadata().id(), "findUserByEmail");
        assert_eq!(queries[1].metadata().id(), "listUsers");
        assert!(
            !queries[0].sql().contains("id: listUsers"),
            "first query sql: {}",
            queries[0].sql()
        );
        assert_eq!(queries[1].sql(), "\nSELECT id FROM users;\n");
    }

    #[test]
    fn rejects_invalid_param_ids_at_param_marker_location() {
        let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: 1bad } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlcomp_query_blocks(source).expect_err("invalid Param id should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");
        let range = diagnostic
            .location()
            .and_then(core::SourceLocation::range)
            .expect("Param diagnostic should include source range");

        assert_eq!(
            diagnostic.message(),
            "invalid Param id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"
        );
        assert_eq!(range.start().line(), 8);
    }

    #[test]
    fn rejects_invalid_inline_param_metadata() {
        for (source, expected_message) in [
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email extra: true } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
                "unknown `param` metadata field `extra`; supported fields are `type`, `id`, `valueType`, and `nullable`",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd id: email } */;
",
                "unknown `paramEnd` metadata field `id`; supported fields are `type`",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: param_end } */;
",
                "unsupported `@sqlcomp` annotation type `param_end`; use `paramEnd` for Param end markers",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: banana } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
                "unsupported Param valueType `banana`; supported values are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: unknown } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
                "unsupported Param valueType `unknown`; supported values are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`",
            ),
        ] {
            let source = source
                .strip_prefix('\n')
                .expect("raw SQL test source should start with a newline");
            let report =
                split_sqlcomp_query_blocks(source).expect_err("invalid Param metadata rejected");

            assert_eq!(diagnostic_messages(&report), [expected_message]);
        }
    }

    #[test]
    fn rejects_unpaired_or_nested_inline_param_markers() {
        for (source, expected_message) in [
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test';
",
                "`param` marker is missing a matching `paramEnd` marker",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = 'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
                "`paramEnd` marker has no matching `param` marker",
            ),
            (
                r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  COALESCE(/* @sqlcomp { type: param id: fallbackEmail } */ 'test@example.test'
  /* @sqlcomp { type: paramEnd } */)
  /* @sqlcomp { type: paramEnd } */;
",
                "nested Param ranges are not supported",
            ),
            (
                r"
/* @sqlcomp { type: param id: email } */
'test@example.test'
/* @sqlcomp { type: paramEnd } */
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users;
",
                "Param markers must appear inside a query body",
            ),
        ] {
            let source = source
                .strip_prefix('\n')
                .expect("raw SQL test source should start with a newline");
            let report = split_sqlcomp_query_blocks(source)
                .expect_err("invalid Param marker structure should be rejected");

            assert_eq!(diagnostic_messages(&report), [expected_message]);
        }
    }

    #[test]
    fn filesystem_source_reader_reads_included_files_as_query_blocks() {
        let project_dir = test_project_dir("reads-included-files");
        let sql_dir = project_dir.join("sql");
        fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
        fs::write(
            sql_dir.join("users.sql"),
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
/* @sqlcomp
{
  type: query
  id: findUser
  cardinality: one
}
*/
SELECT id FROM users WHERE id = 1;
"
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline"),
        )
        .expect("test SQL file should be written");

        let source_read = FileSystemSourceReader
            .read(&compilation_plan(
                &project_dir,
                vec![project_dir.join("sql/**/*.sql")],
                Vec::new(),
            ))
            .expect("included SQL file should be read");
        let queries = source_read.queries();

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].metadata().id(), "listUsers");
        assert_eq!(queries[0].metadata().cardinality(), None);
        assert_eq!(queries[0].sql(), "\nSELECT id FROM users;\n");
        assert_eq!(queries[1].metadata().id(), "findUser");
        assert_eq!(
            queries[1].metadata().cardinality(),
            Some(core::Cardinality::One)
        );
        assert_eq!(queries[1].sql(), "\nSELECT id FROM users WHERE id = 1;\n");

        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    #[test]
    fn filesystem_source_reader_attaches_file_path_to_query_locations() {
        let project_dir = test_project_dir("attaches-query-locations");
        let sql_dir = project_dir.join("sql");
        let sql_path = sql_dir.join("users.sql");
        fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
        fs::write(
            &sql_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline"),
        )
        .expect("test SQL file should be written");

        let source_read = FileSystemSourceReader
            .read(&compilation_plan(
                &project_dir,
                vec![project_dir.join("sql/**/*.sql")],
                Vec::new(),
            ))
            .expect("included SQL file should be read");
        let queries = source_read.queries();
        let location = queries[0]
            .source_location()
            .expect("query should include source location");
        let range = location
            .range()
            .expect("query should include SQL body range");

        assert_eq!(location.path(), Some(sql_path.as_path()));
        assert_eq!(range.start().line(), 7);
        assert_eq!(range.start().column(), 1);
        assert_eq!(queries[0].source_path(), Some(Path::new("sql/users.sql")));

        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    #[test]
    fn source_reader_locations_feed_mysql_parser_diagnostics() {
        let project_dir = test_project_dir("feeds-parser-diagnostics");
        let sql_dir = project_dir.join("sql");
        let sql_path = sql_dir.join("users.sql");
        fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
        fs::write(
            &sql_path,
            r"
/* @sqlcomp
{
  type: query
  id: brokenQuery
}
*/
SELECT FROM;
"
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline"),
        )
        .expect("test SQL file should be written");

        let source_read = FileSystemSourceReader
            .read(&compilation_plan(
                &project_dir,
                vec![project_dir.join("sql/**/*.sql")],
                Vec::new(),
            ))
            .expect("included SQL file should be read");
        let queries = source_read.queries();
        let report = MysqlDialectAnalyzer
            .analyze(&queries[0])
            .expect_err("invalid SQL should produce a parser diagnostic");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("parser diagnostic should be returned");
        let location = diagnostic
            .location()
            .expect("parser diagnostic should include source location");
        let range = location
            .range()
            .expect("parser diagnostic should include source range");

        assert!(
            diagnostic
                .message()
                .starts_with("failed to parse MySQL SQL:"),
            "message: {}",
            diagnostic.message()
        );
        assert_eq!(location.path(), Some(sql_path.as_path()));
        assert_eq!(range.start().line(), 7);
        assert_eq!(range.start().column(), 1);

        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    #[test]
    fn rejects_malformed_hjson_metadata() {
        let source = r"
/* @sqlcomp
{
  type query
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
            .expect_err("malformed Hjson should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");

        assert!(
            diagnostic
                .message()
                .starts_with("failed to parse `@sqlcomp` metadata as Hjson:")
        );
        assert!(diagnostic.location().is_some());
    }

    #[test]
    fn rejects_unsupported_annotation_types() {
        let source = r"
/* @sqlcomp
{
  type: param
  id: userId
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
            .expect_err("unsupported annotation type should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");

        assert_eq!(
            diagnostic.message(),
            "unsupported `@sqlcomp` annotation type `param`; expected `query` metadata"
        );
        assert!(diagnostic.location().is_some());
    }

    #[test]
    fn rejects_invalid_query_ids() {
        for id in ["1bad", "list-users", "\"\""] {
            let source = format!(
                r"
/* @sqlcomp
{{
  type: query
  id: {id}
}}
*/
SELECT 1;
"
            );
            let source = source
                .strip_prefix('\n')
                .expect("raw SQL test source should start with a newline");
            let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
            let report = parse_sqlcomp_query_metadata(&scan.blocks()[0])
                .expect_err("invalid query id should be rejected");
            let diagnostic = report
                .diagnostics()
                .first()
                .expect("a diagnostic should be returned");
            let displayed_id = id.trim_matches('"');

            assert_eq!(
                diagnostic.message(),
                format!("invalid query id `{displayed_id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`")
            );
            assert!(diagnostic.location().is_some());
        }
    }

    #[test]
    fn ignores_marker_like_text_inside_sql_strings() {
        let source = r#"
SELECT '/* @sqlcomp { id: nope } */' AS literal, "/* @sqlcomp */" AS double_quoted;
"#
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("string literal should scan");

        assert!(scan.blocks().is_empty());
        assert_eq!(scan.sql_without_sqlcomp_blocks(), source);
    }

    #[test]
    fn ignores_marker_like_text_inside_line_comments() {
        let source = r"
-- /* @sqlcomp { id: nope } */
SELECT 1;
# /* @sqlcomp */
SELECT 2;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlcomp_blocks(source).expect("line comments should scan");

        assert!(scan.blocks().is_empty());
        assert_eq!(scan.sql_without_sqlcomp_blocks(), source);
    }

    #[test]
    fn rejects_unterminated_block_comment() {
        let report = scan_sqlcomp_blocks(
            r"
SELECT 1;
/* @sqlcomp
{ id: broken }
"
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline"),
        )
        .expect_err("unterminated block comment should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");
        let location = diagnostic
            .location()
            .expect("unterminated comment should include location");
        let range = location
            .range()
            .expect("unterminated comment should include source range");

        assert_eq!(diagnostic.message(), "unterminated SQL block comment");
        assert_eq!(range.start().line(), 2);
        assert_eq!(range.start().column(), 1);
    }

    #[test]
    fn source_reader_rejects_duplicate_query_ids_in_the_same_file() {
        let project_dir = test_project_dir("duplicate-same-file");
        let source_path = project_dir.join("sql").join("users.sql");
        write_sql(
            &source_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;

/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
        );
        let plan = compilation_plan(&project_dir, vec![source_path.clone()], Vec::new());

        let report = FileSystemSourceReader
            .read(&plan)
            .expect_err("duplicate query ids should be rejected");

        assert_duplicate_query_report(&report, &source_path);
        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    #[test]
    fn source_reader_rejects_duplicate_query_ids_across_files() {
        let project_dir = test_project_dir("duplicate-across-files");
        let first_path = project_dir.join("sql").join("first.sql");
        let second_path = project_dir.join("sql").join("second.sql");
        write_sql(
            &first_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
        );
        write_sql(
            &second_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
        );
        let plan = compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        );

        let report = FileSystemSourceReader
            .read(&plan)
            .expect_err("duplicate query ids should be rejected");

        assert_duplicate_query_report(&report, &second_path);
        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    #[test]
    fn source_reader_collects_independent_source_intake_diagnostics_across_files() {
        let project_dir = test_project_dir("aggregates-source-intake-diagnostics");
        let exec_path = project_dir.join("sql").join("01_exec_cardinality.sql");
        let first_duplicate_path = project_dir.join("sql").join("02_duplicate_first.sql");
        let second_duplicate_path = project_dir.join("sql").join("03_duplicate_second.sql");
        write_sql(
            &exec_path,
            r"
/* @sqlcomp
{
  type: query
  id: execQuery
  cardinality: exec
}
*/
SELECT id FROM users;
",
        );
        write_sql(
            &first_duplicate_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
        );
        write_sql(
            &second_duplicate_path,
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
        );
        let plan = compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        );

        let report = FileSystemSourceReader
            .read(&plan)
            .expect_err("source intake diagnostics should be aggregated");

        assert_eq!(
            diagnostic_messages(&report),
            [
                "`cardinality: exec` is reserved for future non-SELECT support and is not supported in the MVP",
                "duplicate query id `listUsers`; query IDs must be unique across the full compile run",
                "first declared here",
            ]
        );
        assert_eq!(
            report.diagnostics()[0]
                .location()
                .and_then(core::SourceLocation::path),
            Some(exec_path.as_path())
        );
        assert_eq!(
            report.diagnostics()[1]
                .location()
                .and_then(core::SourceLocation::path),
            Some(second_duplicate_path.as_path())
        );
        assert_eq!(
            report.diagnostics()[2]
                .location()
                .and_then(core::SourceLocation::path),
            Some(first_duplicate_path.as_path())
        );

        fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    }

    fn assert_duplicate_query_report(report: &core::DiagnosticReport, duplicate_path: &Path) {
        assert_eq!(report.diagnostics().len(), 2);
        assert_eq!(
            report.diagnostics()[0].message(),
            "duplicate query id `listUsers`; query IDs must be unique across the full compile run"
        );
        assert_eq!(
            report.diagnostics()[0]
                .location()
                .and_then(core::SourceLocation::path),
            Some(duplicate_path)
        );
        assert_eq!(report.diagnostics()[1].message(), "first declared here");
    }

    fn diagnostic_messages(report: &core::DiagnosticReport) -> Vec<&str> {
        report
            .diagnostics()
            .iter()
            .map(core::Diagnostic::message)
            .collect()
    }

    fn compilation_plan(
        config_dir: &Path,
        source_include: Vec<PathBuf>,
        source_exclude: Vec<PathBuf>,
    ) -> core::CompilationPlan {
        core::CompilationPlan::new(
            config_dir.to_path_buf(),
            source_include,
            source_exclude,
            config_dir.join("generated"),
            core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            core::TargetConfig::new(core::TargetLanguage::TypeScript),
        )
    }

    fn write_sql(path: &Path, contents: &str) {
        let contents = contents
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let parent = path.parent().expect("test path should include a parent");
        fs::create_dir_all(parent).expect("temp source dir should be created");
        fs::write(path, contents).expect("temp SQL file should be written");
    }

    fn test_project_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sqlcomp-source-fs-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale test project directory should be removed");
        }
        fs::create_dir_all(&dir).expect("test project directory should be created");
        dir
    }
}
