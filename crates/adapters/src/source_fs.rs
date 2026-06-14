//! Filesystem source intake adapter.

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use sqlcomp_app::SourceReader;
use sqlcomp_core as core;

const SQLCOMP_MARKER: &str = "@sqlcomp";

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlcompBlock {
    payload: String,
    comment_range: core::SourceRange,
    payload_range: core::SourceRange,
    comment_start_byte: usize,
    comment_end_byte: usize,
}

impl SqlcompBlock {
    /// Build a sqlcomp metadata block.
    #[must_use]
    pub const fn new(
        payload: String,
        comment_range: core::SourceRange,
        payload_range: core::SourceRange,
        comment_start_byte: usize,
        comment_end_byte: usize,
    ) -> Self {
        Self {
            payload,
            comment_range,
            payload_range,
            comment_start_byte,
            comment_end_byte,
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

    const fn comment_start_byte(&self) -> usize {
        self.comment_start_byte
    }

    const fn comment_end_byte(&self) -> usize {
        self.comment_end_byte
    }
}

/// Scan SQL source for canonical `@sqlcomp` block comments.
///
/// # Errors
///
/// Returns a diagnostic when a SQL block comment is not terminated.
pub fn scan_sqlcomp_blocks(source: &str) -> core::DiagnosticResult<SqlcompBlockScan> {
    Scanner::new(source).scan()
}

/// Parse one discovered `@sqlcomp` block as MVP query metadata.
///
/// # Errors
///
/// Returns diagnostics when the payload is malformed Hjson or declares an
/// annotation type outside the MVP query-only scope.
pub fn parse_sqlcomp_query_metadata(
    block: &SqlcompBlock,
) -> core::DiagnosticResult<core::QueryMetadata> {
    let raw = deser_hjson::from_str::<RawSqlcompMetadata>(block.payload()).map_err(|error| {
        metadata_error(
            format!("failed to parse `@sqlcomp` metadata as Hjson: {error}"),
            block.payload_range(),
        )
    })?;
    let Some(annotation_type) = raw.annotation_type else {
        return Err(metadata_error(
            "missing required `@sqlcomp` metadata field `type`",
            block.payload_range(),
        ));
    };

    if annotation_type != "query" {
        return Err(metadata_error(
            format!(
                "unsupported `@sqlcomp` annotation type `{annotation_type}`; MVP only supports `query`"
            ),
            block.payload_range(),
        ));
    }

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

/// Dummy filesystem-backed source reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemSourceReader;

impl SourceReader for FileSystemSourceReader {
    fn read(&self, plan: &core::CompilationPlan) -> core::DiagnosticResult<Vec<core::RawQuery>> {
        let source_paths = discover_source_files(plan)?;
        let mut seen_ids = HashMap::new();
        let mut queries = Vec::new();

        for source_path in source_paths {
            let source = fs::read_to_string(&source_path).map_err(|error| {
                core::DiagnosticReport::new(
                    core::Diagnostic::error(format!(
                        "failed to read SQL source file `{}`: {error}",
                        source_path.display()
                    ))
                    .with_location(core::SourceLocation::for_path(source_path.clone())),
                )
            })?;
            queries.extend(read_queries_from_source(
                &source_path,
                &source,
                &mut seen_ids,
            )?);
        }

        Ok(queries)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSqlcompMetadata {
    #[serde(rename = "type")]
    annotation_type: Option<String>,
    id: Option<String>,
    cardinality: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueryDeclaration {
    path: PathBuf,
    range: core::SourceRange,
}

type SeenQueryIds = HashMap<String, QueryDeclaration>;

fn discover_source_files(plan: &core::CompilationPlan) -> core::DiagnosticResult<Vec<PathBuf>> {
    let mut source_paths = Vec::new();

    for include in plan.source_include() {
        source_paths.extend(discover_include_pattern(include)?);
    }

    source_paths.sort();
    source_paths.dedup();
    source_paths.retain(|path| {
        !plan
            .source_exclude()
            .iter()
            .any(|exclude| path_matches_pattern(path, exclude))
    });

    Ok(source_paths)
}

fn discover_include_pattern(pattern: &Path) -> core::DiagnosticResult<Vec<PathBuf>> {
    if !path_contains_glob(pattern) {
        return Ok(if pattern.is_file() {
            vec![pattern.to_path_buf()]
        } else {
            Vec::new()
        });
    }

    let base_dir = glob_base_dir(pattern);
    if !base_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    collect_matching_files(&base_dir, pattern, &mut paths)?;
    Ok(paths)
}

fn collect_matching_files(
    directory: &Path,
    pattern: &Path,
    paths: &mut Vec<PathBuf>,
) -> core::DiagnosticResult<()> {
    let entries = fs::read_dir(directory).map_err(|error| {
        core::DiagnosticReport::new(
            core::Diagnostic::error(format!(
                "failed to read source directory `{}`: {error}",
                directory.display()
            ))
            .with_location(core::SourceLocation::for_path(directory)),
        )
    })?;

    for entry in entries {
        let path = entry
            .map_err(|error| {
                core::DiagnosticReport::new(core::Diagnostic::error(format!(
                    "failed to read source directory entry in `{}`: {error}",
                    directory.display()
                )))
            })?
            .path();

        if path.is_dir() {
            collect_matching_files(&path, pattern, paths)?;
        } else if path.is_file() && path_matches_pattern(&path, pattern) {
            paths.push(path);
        }
    }

    Ok(())
}

fn read_queries_from_source(
    source_path: &Path,
    source: &str,
    seen_ids: &mut SeenQueryIds,
) -> core::DiagnosticResult<Vec<core::RawQuery>> {
    let scan = scan_sqlcomp_blocks(source).map_err(|report| report_with_path(&report, source_path));
    let scan = scan?;
    let mut queries = Vec::new();

    for (index, block) in scan.blocks().iter().enumerate() {
        let metadata = parse_sqlcomp_query_metadata(block)
            .map_err(|report| report_with_path(&report, source_path))?;
        reject_duplicate_query_id(source_path, block, &metadata, seen_ids)?;

        let sql_start = block.comment_end_byte();
        let sql_end = scan
            .blocks()
            .get(index + 1)
            .map_or(source.len(), SqlcompBlock::comment_start_byte);
        queries.push(core::RawQuery::new(
            source_path.to_path_buf(),
            source_range_for_span(source, sql_start, sql_end),
            metadata,
            source[sql_start..sql_end].to_owned(),
        ));
    }

    Ok(queries)
}

fn reject_duplicate_query_id(
    source_path: &Path,
    block: &SqlcompBlock,
    metadata: &core::QueryMetadata,
    seen_ids: &mut SeenQueryIds,
) -> core::DiagnosticResult<()> {
    let declaration = QueryDeclaration {
        path: source_path.to_path_buf(),
        range: block.payload_range(),
    };

    if let Some(first_declaration) = seen_ids.insert(metadata.id().to_owned(), declaration) {
        return Err(core::DiagnosticReport::from_diagnostics(vec![
            core::Diagnostic::error(format!(
                "duplicate query id `{}`; query IDs must be unique across the full compile run",
                metadata.id()
            ))
            .with_location(core::SourceLocation::at_range(
                source_path,
                block.payload_range(),
            )),
            core::Diagnostic::note("first declared here").with_location(
                core::SourceLocation::at_range(first_declaration.path, first_declaration.range),
            ),
        ]));
    }

    Ok(())
}

fn report_with_path(report: &core::DiagnosticReport, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::from_diagnostics(
        report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic_with_path(diagnostic, path))
            .collect(),
    )
}

fn diagnostic_with_path(diagnostic: &core::Diagnostic, path: &Path) -> core::Diagnostic {
    let mut next = core::Diagnostic::new(diagnostic.severity(), diagnostic.message().to_owned());
    if let Some(range) = diagnostic.location().and_then(core::SourceLocation::range) {
        next = next.with_location(core::SourceLocation::at_range(path, range));
    } else {
        next = next.with_location(core::SourceLocation::for_path(path));
    }

    next
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

            self.blocks.push(SqlcompBlock::new(
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

fn path_contains_glob(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .chars()
            .any(|char| matches!(char, '*' | '?'))
    })
}

fn glob_base_dir(pattern: &Path) -> PathBuf {
    let mut base_dir = PathBuf::new();

    for component in pattern.components() {
        if component_contains_glob(component) {
            break;
        }

        base_dir.push(component.as_os_str());
    }

    if base_dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        base_dir
    }
}

fn component_contains_glob(component: Component<'_>) -> bool {
    component
        .as_os_str()
        .to_string_lossy()
        .chars()
        .any(|char| matches!(char, '*' | '?'))
}

fn path_matches_pattern(path: &Path, pattern: &Path) -> bool {
    let path_segments = path_match_segments(path);
    let pattern_segments = path_match_segments(pattern);

    match_segments(&path_segments, &pattern_segments)
}

fn path_match_segments(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir => Some(String::from("/")),
            Component::CurDir => None,
            Component::ParentDir => Some(String::from("..")),
            Component::Normal(segment) => Some(segment.to_string_lossy().into_owned()),
        })
        .collect()
}

fn match_segments(path_segments: &[String], pattern_segments: &[String]) -> bool {
    if let Some((pattern_segment, remaining_pattern)) = pattern_segments.split_first() {
        if pattern_segment == "**" {
            return match_segments(path_segments, remaining_pattern)
                || path_segments
                    .split_first()
                    .is_some_and(|(_, remaining_path)| {
                        match_segments(remaining_path, pattern_segments)
                    });
        }

        return path_segments
            .split_first()
            .is_some_and(|(path_segment, remaining_path)| {
                segment_matches(path_segment, pattern_segment)
                    && match_segments(remaining_path, remaining_pattern)
            });
    }

    path_segments.is_empty()
}

fn segment_matches(text: &str, pattern: &str) -> bool {
    let text_chars = text.chars().collect::<Vec<_>>();
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let mut matches = vec![false; text_chars.len() + 1];
    matches[0] = true;

    for pattern_char in pattern_chars {
        let mut next = vec![false; text_chars.len() + 1];

        if pattern_char == '*' {
            next[0] = matches[0];
            for index in 1..=text_chars.len() {
                next[index] = matches[index] || next[index - 1];
            }
        } else {
            for (index, text_char) in text_chars.iter().enumerate() {
                next[index + 1] =
                    matches[index] && (pattern_char == '?' || pattern_char == *text_char);
            }
        }

        matches = next;
    }

    matches[text_chars.len()]
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
    use std::path::{Path, PathBuf};

    use super::{FileSystemSourceReader, parse_sqlcomp_query_metadata, scan_sqlcomp_blocks};
    use sqlcomp_app::SourceReader;
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
            "unsupported `@sqlcomp` annotation type `param`; MVP only supports `query`"
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
    fn source_reader_extracts_multiple_query_blocks_from_included_files() {
        let project_dir = unique_temp_dir("sqlcomp-source-reader-multiple");
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
  id: findUser
  cardinality: one
}
*/
SELECT id FROM users WHERE id = 1;
",
        );
        let plan = compilation_plan(project_dir.clone(), vec![source_path.clone()], Vec::new());

        let queries = FileSystemSourceReader
            .read(&plan)
            .expect("valid source files should be read");

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].metadata().id(), "listUsers");
        assert_eq!(queries[0].metadata().cardinality(), None);
        assert_eq!(queries[0].sql().trim(), "SELECT id FROM users;");
        assert_eq!(queries[0].source_path(), source_path.as_path());
        assert_eq!(queries[1].metadata().id(), "findUser");
        assert_eq!(
            queries[1].metadata().cardinality(),
            Some(core::Cardinality::One)
        );
        assert_eq!(
            queries[1].sql().trim(),
            "SELECT id FROM users WHERE id = 1;"
        );

        remove_temp_dir(project_dir);
    }

    #[test]
    fn source_reader_rejects_duplicate_query_ids_in_the_same_file() {
        let project_dir = unique_temp_dir("sqlcomp-source-reader-duplicate-same-file");
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
        let plan = compilation_plan(project_dir.clone(), vec![source_path.clone()], Vec::new());

        let report = FileSystemSourceReader
            .read(&plan)
            .expect_err("duplicate query ids should be rejected");

        assert_duplicate_query_report(&report, &source_path);
        remove_temp_dir(project_dir);
    }

    #[test]
    fn source_reader_rejects_duplicate_query_ids_across_files() {
        let project_dir = unique_temp_dir("sqlcomp-source-reader-duplicate-across-files");
        let active_path = project_dir.join("sql").join("active.sql");
        let archived_path = project_dir.join("sql").join("archived.sql");
        write_sql(
            &active_path,
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
            &archived_path,
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
            project_dir.clone(),
            vec![active_path, archived_path.clone()],
            Vec::new(),
        );

        let report = FileSystemSourceReader
            .read(&plan)
            .expect_err("duplicate query ids should be rejected");

        assert_duplicate_query_report(&report, &archived_path);
        remove_temp_dir(project_dir);
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

    fn compilation_plan(
        config_dir: PathBuf,
        source_include: Vec<PathBuf>,
        source_exclude: Vec<PathBuf>,
    ) -> core::CompilationPlan {
        core::CompilationPlan::new(
            config_dir,
            source_include,
            source_exclude,
            PathBuf::from("generated"),
            core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            core::TargetConfig::new(core::TargetLanguage::TypeScript),
        )
    }

    fn write_sql(path: &Path, contents: &str) {
        let contents = contents
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let parent = path.parent().expect("test path should include a parent");
        std::fs::create_dir_all(parent).expect("temp source dir should be created");
        std::fs::write(path, contents).expect("temp SQL file should be written");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
    }

    fn remove_temp_dir(path: PathBuf) {
        std::fs::remove_dir_all(path).expect("temp source tree should be removed");
    }
}
