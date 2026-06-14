//! Filesystem source intake adapter.

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
}

impl SqlcompBlock {
    /// Build a sqlcomp metadata block.
    #[must_use]
    pub const fn new(
        payload: String,
        comment_range: core::SourceRange,
        payload_range: core::SourceRange,
    ) -> Self {
        Self {
            payload,
            comment_range,
            payload_range,
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

    if raw.annotation_type != "query" {
        return Err(metadata_error(
            format!(
                "unsupported `@sqlcomp` annotation type `{}`; MVP only supports `query`",
                raw.annotation_type
            ),
            block.payload_range(),
        ));
    }

    if !is_valid_query_id(&raw.id) {
        return Err(metadata_error(
            format!(
                "invalid query id `{}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
                raw.id
            ),
            block.payload_range(),
        ));
    }

    Ok(core::QueryMetadata::new(
        raw.id,
        raw.cardinality.map(core::Cardinality::from),
    ))
}

/// Dummy filesystem-backed source reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemSourceReader;

impl SourceReader for FileSystemSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> core::DiagnosticResult<Vec<core::RawQuery>> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSqlcompMetadata {
    #[serde(rename = "type")]
    annotation_type: String,
    id: String,
    cardinality: Option<RawCardinality>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RawCardinality {
    One,
    Many,
}

impl From<RawCardinality> for core::Cardinality {
    fn from(value: RawCardinality) -> Self {
        match value {
            RawCardinality::One => Self::One,
            RawCardinality::Many => Self::Many,
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

            self.blocks
                .push(SqlcompBlock::new(payload, comment_range, payload_range));
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
    use super::{parse_sqlcomp_query_metadata, scan_sqlcomp_blocks};
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
        let source = "/* @sqlcomp\n{ type: query, id: listUsers }\n*/\nSELECT id FROM users;\n";
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
        let source = "/* @sqlcomp\n{ id: first }\n*/\nSELECT 1;\n/* @sqlcomp\n{ id: second }\n*/\nSELECT 2;\n";
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
        let source = "/* @sqlcomp\n{\n  type: query\n  id: listUsers\n  cardinality: one\n}\n*/\nSELECT id FROM users;\n";
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let metadata =
            parse_sqlcomp_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

        assert_eq!(metadata.id(), "listUsers");
        assert_eq!(metadata.cardinality(), Some(core::Cardinality::One));
    }

    #[test]
    fn parses_query_metadata_without_optional_cardinality() {
        let source =
            "/* @sqlcomp\n{\n  type: query\n  id: listUsers\n}\n*/\nSELECT id FROM users;\n";
        let scan = scan_sqlcomp_blocks(source).expect("annotated SQL should scan");
        let metadata =
            parse_sqlcomp_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

        assert_eq!(metadata.id(), "listUsers");
        assert_eq!(metadata.cardinality(), None);
    }

    #[test]
    fn rejects_malformed_hjson_metadata() {
        let source = "/* @sqlcomp\n{\n  type query\n}\n*/\nSELECT id FROM users;\n";
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
        let source = "/* @sqlcomp\n{\n  type: param\n  id: userId\n}\n*/\nSELECT id FROM users;\n";
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
            let source = format!("/* @sqlcomp\n{{\n  type: query\n  id: {id}\n}}\n*/\nSELECT 1;\n");
            let scan = scan_sqlcomp_blocks(&source).expect("annotated SQL should scan");
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
        let source = "SELECT '/* @sqlcomp { id: nope } */' AS literal, \"/* @sqlcomp */\" AS double_quoted;\n";
        let scan = scan_sqlcomp_blocks(source).expect("string literal should scan");

        assert!(scan.blocks().is_empty());
        assert_eq!(scan.sql_without_sqlcomp_blocks(), source);
    }

    #[test]
    fn ignores_marker_like_text_inside_line_comments() {
        let source = "-- /* @sqlcomp { id: nope } */\nSELECT 1;\n# /* @sqlcomp */\nSELECT 2;\n";
        let scan = scan_sqlcomp_blocks(source).expect("line comments should scan");

        assert!(scan.blocks().is_empty());
        assert_eq!(scan.sql_without_sqlcomp_blocks(), source);
    }

    #[test]
    fn rejects_unterminated_block_comment() {
        let report = scan_sqlcomp_blocks("SELECT 1;\n/* @sqlcomp\n{ id: broken }\n")
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
}
