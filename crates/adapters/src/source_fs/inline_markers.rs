use sqlay_core as core;

use crate::source_fs::diagnostics::metadata_error;
use crate::source_fs::metadata::{ParsedSqlayBlock, SqlayAnnotation};
use crate::source_fs::scanner::{
    SqlayBlock, is_quote_delimiter, source_range_for_span, source_range_for_sql_body,
};

const RAW_PLACEHOLDER_GUIDANCE: &str = "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression, such as `/* @sqlay { type: param id: value } */ 1 /* @sqlay { type: paramEnd } */`";

pub(super) struct InlineMarkerReplacement {
    pub(super) analysis_sql: String,
    pub(super) param_usages: Vec<core::ParamUsage>,
    pub(super) slot_usages: Vec<core::SlotUsage>,
}

pub(super) fn replace_inline_markers(
    source: &str,
    body_start: usize,
    body_end: usize,
    parsed_blocks: &[ParsedSqlayBlock<'_>],
) -> core::DiagnosticResult<InlineMarkerReplacement> {
    let query_blocks = parsed_blocks
        .iter()
        .filter(|parsed_block| {
            let start = parsed_block.block.comment_start_index();
            start >= body_start && start < body_end
        })
        .collect::<Vec<_>>();
    let mut analysis_sql = String::with_capacity(body_end - body_start);
    let mut param_usages = Vec::new();
    let mut slot_usages = Vec::new();
    let mut cursor = body_start;
    let mut index = 0;

    while index < query_blocks.len() {
        let parsed_block = query_blocks[index];
        match &parsed_block.annotation {
            SqlayAnnotation::Query(_) | SqlayAnnotation::Mutation(_) => {
                index += 1;
            }
            SqlayAnnotation::Fragment(_) => {
                unreachable!("fragment annotations are global source unit boundaries");
            }
            SqlayAnnotation::Slot(metadata) => {
                append_non_param_sql(
                    source,
                    cursor,
                    parsed_block.block.comment_start_index(),
                    &mut analysis_sql,
                )?;

                slot_usages.push(core::SlotUsage::new(
                    metadata.id.clone(),
                    metadata.targets.clone(),
                    analysis_sql.len(),
                    core::SourceLocation::from_range(parsed_block.block.comment_range()),
                ));

                cursor = parsed_block.block.comment_end_index();
                index += 1;
            }
            SqlayAnnotation::Param(metadata) => {
                append_non_param_sql(
                    source,
                    cursor,
                    parsed_block.block.comment_start_index(),
                    &mut analysis_sql,
                )?;

                let Some(end_block) = query_blocks.get(index + 1).copied() else {
                    unreachable!("Param marker pairing is validated before replacement");
                };
                debug_assert!(matches!(end_block.annotation, SqlayAnnotation::ParamEnd));

                let sample_start = parsed_block.block.comment_end_index();
                let sample_end = end_block.block.comment_start_index();
                reject_sample_placeholder(source, sample_start, sample_end)?;

                let placeholder_index = analysis_sql.len();
                analysis_sql.push('?');
                param_usages.push(
                    core::ParamUsage::new(
                        metadata.id.clone(),
                        metadata.value_type,
                        metadata.nullable,
                        core::SourceLocation::from_range(source_range_for_span(
                            source,
                            parsed_block.block.comment_start_index(),
                            end_block.block.comment_end_index(),
                        )),
                    )
                    .with_placeholder_index(placeholder_index)
                    .with_sample_sql(source[sample_start..sample_end].to_owned()),
                );

                cursor = end_block.block.comment_end_index();
                index += 2;
            }
            SqlayAnnotation::ParamEnd => {
                unreachable!("Param end markers are consumed with their matching Param marker");
            }
        }
    }

    append_non_param_sql(source, cursor, body_end, &mut analysis_sql)?;
    verify_placeholder_count(
        &analysis_sql,
        param_usages.len(),
        source_range_for_sql_body(source, body_start, body_end),
    )?;

    Ok(InlineMarkerReplacement {
        analysis_sql,
        param_usages,
        slot_usages,
    })
}

fn append_non_param_sql(
    source: &str,
    start: usize,
    end: usize,
    output: &mut String,
) -> core::DiagnosticResult<()> {
    reject_raw_placeholder(source, start, end)?;
    output.push_str(&source[start..end]);
    Ok(())
}

fn reject_raw_placeholder(source: &str, start: usize, end: usize) -> core::DiagnosticResult<()> {
    if let Some(index) = first_placeholder_index(source, start, end) {
        return Err(metadata_error(
            RAW_PLACEHOLDER_GUIDANCE,
            source_range_for_span(source, index, index + 1),
        ));
    }

    Ok(())
}

fn reject_sample_placeholder(source: &str, start: usize, end: usize) -> core::DiagnosticResult<()> {
    if let Some(index) = first_placeholder_index(source, start, end) {
        return Err(metadata_error(
            "`?` placeholders are not allowed inside Param sample expressions",
            source_range_for_span(source, index, index + 1),
        ));
    }

    Ok(())
}

fn verify_placeholder_count(
    analysis_sql: &str,
    param_usage_count: usize,
    range: core::SourceRange,
) -> core::DiagnosticResult<()> {
    let placeholder_count = PlaceholderScanner::new(analysis_sql, 0, analysis_sql.len()).count();
    if placeholder_count != param_usage_count {
        return Err(metadata_error(
            format!(
                "generated placeholder count {placeholder_count} does not match Param usage count {param_usage_count}"
            ),
            range,
        ));
    }

    Ok(())
}

pub(super) fn reject_fragment_statement_separator(
    source: &str,
    start: usize,
    end: usize,
) -> core::DiagnosticResult<()> {
    if let Some(index) = first_statement_separator_index(source, start, end) {
        return Err(metadata_error(
            "raw statement separator `;` is not supported in fragment bodies",
            source_range_for_span(source, index, index + 1),
        ));
    }

    Ok(())
}

fn first_statement_separator_index(source: &str, start: usize, end: usize) -> Option<usize> {
    StatementSeparatorScanner::new(source, start, end).next_separator_index()
}

/// Validates the structural constraints of inline `@sqlay` markers.
///
/// Ensures that `param` and `paramEnd` markers are paired without nesting, that inline
/// markers appear only inside query, mutation, or fragment bodies, that `slot`
/// markers are used only in query or mutation bodies, and that `slot` markers do
/// not nest within `param` ranges.
pub(super) fn validate_inline_markers(
    parsed_blocks: &[ParsedSqlayBlock<'_>],
) -> core::DiagnosticResult<()> {
    let mut context = InlineMarkerContext::OutsideSourceUnit;
    let mut open_param_block: Option<&SqlayBlock> = None;

    for parsed_block in parsed_blocks {
        match parsed_block.annotation {
            SqlayAnnotation::Query(_) => {
                if let Some(block) = open_param_block.take() {
                    return Err(metadata_error(
                        "`param` marker is missing a matching `paramEnd` marker",
                        block.payload_range(),
                    ));
                }
                context = InlineMarkerContext::QueryBody;
            }
            SqlayAnnotation::Mutation(_) => {
                if let Some(block) = open_param_block.take() {
                    return Err(metadata_error(
                        "`param` marker is missing a matching `paramEnd` marker",
                        block.payload_range(),
                    ));
                }
                context = InlineMarkerContext::MutationBody;
            }
            SqlayAnnotation::Fragment(_) => {
                if let Some(block) = open_param_block.take() {
                    return Err(metadata_error(
                        "`param` marker is missing a matching `paramEnd` marker",
                        block.payload_range(),
                    ));
                }
                context = InlineMarkerContext::FragmentBody;
            }
            SqlayAnnotation::Param(_) => {
                if context == InlineMarkerContext::OutsideSourceUnit {
                    return Err(metadata_error(
                        "`param` markers must appear inside a query, mutation, or fragment body; top-level Param markers are not supported",
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
            SqlayAnnotation::ParamEnd => {
                if context == InlineMarkerContext::OutsideSourceUnit {
                    return Err(metadata_error(
                        "`paramEnd` markers must appear inside a query, mutation, or fragment body; top-level paramEnd markers are not supported",
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
            SqlayAnnotation::Slot(_) => {
                match context {
                    InlineMarkerContext::OutsideSourceUnit => {
                        return Err(metadata_error(
                            "`slot` markers must appear inside a query or mutation body; top-level Slot markers are not supported",
                            parsed_block.block.payload_range(),
                        ));
                    }
                    InlineMarkerContext::FragmentBody => {
                        return Err(metadata_error(
                            "slot markers inside fragments are not supported yet; define slots in query or mutation bodies",
                            parsed_block.block.payload_range(),
                        ));
                    }
                    InlineMarkerContext::QueryBody | InlineMarkerContext::MutationBody => {}
                }
                if open_param_block.is_some() {
                    return Err(metadata_error(
                        "Slot markers are not supported inside Param ranges",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InlineMarkerContext {
    OutsideSourceUnit,
    QueryBody,
    MutationBody,
    FragmentBody,
}

fn first_placeholder_index(source: &str, start: usize, end: usize) -> Option<usize> {
    PlaceholderScanner::new(source, start, end).next_placeholder_index()
}

struct PlaceholderScanner<'a> {
    source: &'a str,
    index: usize,
    end: usize,
}

impl<'a> PlaceholderScanner<'a> {
    const fn new(source: &'a str, start: usize, end: usize) -> Self {
        Self {
            source,
            index: start,
            end,
        }
    }

    fn count(mut self) -> usize {
        let mut count = 0;
        while self.next_placeholder_index().is_some() {
            count += 1;
        }

        count
    }

    fn next_placeholder_index(&mut self) -> Option<usize> {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(is_quote_delimiter) {
                self.skip_quoted();
            } else if self.current_char() == Some('?') {
                let index = self.index;
                self.advance_current();
                return Some(index);
            } else {
                self.advance_current();
            }
        }

        None
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

    fn skip_quoted(&mut self) {
        let delimiter = self
            .current_char()
            .expect("quoted skip should start at a delimiter");
        self.advance_current();

        while let Some(char) = self.current_char() {
            self.advance_current();

            if delimiter != '`' && char == '\\' {
                if !self.is_at_end() {
                    self.advance_current();
                }
                continue;
            }

            if char == delimiter {
                if self.current_char() == Some(delimiter) {
                    self.advance_current();
                } else {
                    break;
                }
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        if self.is_at_end() {
            return None;
        }

        self.source[self.index..self.end].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.end
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..self.end].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..self.end]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}

struct StatementSeparatorScanner<'a> {
    source: &'a str,
    index: usize,
    end: usize,
}

impl<'a> StatementSeparatorScanner<'a> {
    const fn new(source: &'a str, start: usize, end: usize) -> Self {
        Self {
            source,
            index: start,
            end,
        }
    }

    fn next_separator_index(&mut self) -> Option<usize> {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(is_quote_delimiter) {
                self.skip_quoted();
            } else if self.current_char() == Some(';') {
                let index = self.index;
                self.advance_current();
                return Some(index);
            } else {
                self.advance_current();
            }
        }

        None
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

    fn skip_quoted(&mut self) {
        let delimiter = self
            .current_char()
            .expect("quoted skip should start at a delimiter");
        self.advance_current();

        while let Some(char) = self.current_char() {
            self.advance_current();

            if delimiter != '`' && char == '\\' {
                if !self.is_at_end() {
                    self.advance_current();
                }
                continue;
            }

            if char == delimiter {
                if self.current_char() == Some(delimiter) {
                    self.advance_current();
                } else {
                    break;
                }
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        if self.is_at_end() {
            return None;
        }

        self.source[self.index..self.end].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.end
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..self.end].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..self.end]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}
