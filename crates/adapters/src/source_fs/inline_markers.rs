use sqlay_core as core;

mod scanners;

use crate::source_fs::diagnostics::metadata_error;
use crate::source_fs::metadata::{ParsedSqlayBlock, SqlayAnnotation};
use crate::source_fs::scanner::{SqlayBlock, source_range_for_span, source_range_for_sql_body};

use scanners::{first_placeholder_index, first_statement_separator_index, placeholder_count};

const RAW_PLACEHOLDER_GUIDANCE: &str = "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression, such as `/* @sqlay { type: param id: value } */ 1 /* @sqlay { type: paramEnd } */`";

pub(super) struct InlineMarkerReplacement {
    pub(super) analysis_sql: String,
    pub(super) param_usages: Vec<core::ParamUsage>,
    pub(super) slot_usages: Vec<core::SlotUsage>,
    pub(super) repeat_usages: Vec<core::RepeatUsage>,
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
    let mut state = ReplacementState::new(source, body_start, body_end - body_start);
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
                state.append_slot(parsed_block, &metadata.id, &metadata.targets)?;
                index += 1;
            }
            SqlayAnnotation::Param(metadata) => {
                let Some(end_block) = query_blocks.get(index + 1).copied() else {
                    unreachable!("Param marker pairing is validated before replacement");
                };
                debug_assert!(matches!(end_block.annotation, SqlayAnnotation::ParamEnd));
                state.append_param(
                    parsed_block,
                    end_block,
                    &metadata.id,
                    metadata.value_type,
                    metadata.nullable,
                )?;
                index += 2;
            }
            SqlayAnnotation::ParamEnd => {
                unreachable!("Param end markers are consumed with their matching Param marker");
            }
            SqlayAnnotation::Repeat(metadata) => {
                state.open_repeat(parsed_block, &metadata.id, &metadata.separator)?;
                index += 1;
            }
            SqlayAnnotation::RepeatEnd => {
                state.close_repeat(parsed_block)?;
                index += 1;
            }
        }
    }

    state.finish(
        body_end,
        source_range_for_sql_body(source, body_start, body_end),
    )
}

struct ReplacementState<'a> {
    source: &'a str,
    analysis_sql: String,
    param_usages: Vec<core::ParamUsage>,
    slot_usages: Vec<core::SlotUsage>,
    repeat_usages: Vec<core::RepeatUsage>,
    open_repeat: Option<PendingRepeat<'a>>,
    cursor: usize,
}

impl<'a> ReplacementState<'a> {
    fn new(source: &'a str, cursor: usize, capacity: usize) -> Self {
        Self {
            source,
            analysis_sql: String::with_capacity(capacity),
            param_usages: Vec::new(),
            slot_usages: Vec::new(),
            repeat_usages: Vec::new(),
            open_repeat: None,
            cursor,
        }
    }

    fn append_slot(
        &mut self,
        parsed_block: &ParsedSqlayBlock<'a>,
        id: &str,
        targets: &[String],
    ) -> core::DiagnosticResult<()> {
        debug_assert!(self.open_repeat.is_none());
        self.append_non_param_sql(parsed_block.block.comment_start_index())?;
        self.slot_usages.push(core::SlotUsage::new(
            id.to_owned(),
            targets.to_vec(),
            self.analysis_sql.len(),
            core::SourceLocation::from_range(parsed_block.block.comment_range()),
        ));
        self.cursor = parsed_block.block.comment_end_index();
        Ok(())
    }

    fn append_param(
        &mut self,
        parsed_block: &ParsedSqlayBlock<'a>,
        end_block: &ParsedSqlayBlock<'a>,
        id: &str,
        value_type: Option<core::CoreType>,
        nullable: bool,
    ) -> core::DiagnosticResult<()> {
        self.append_non_param_sql(parsed_block.block.comment_start_index())?;
        let sample_start = parsed_block.block.comment_end_index();
        let sample_end = end_block.block.comment_start_index();
        reject_sample_placeholder(self.source, sample_start, sample_end)?;

        let placeholder_index = self.analysis_sql.len();
        self.analysis_sql.push('?');
        let usage = core::ParamUsage::new(
            id.to_owned(),
            value_type,
            nullable,
            core::SourceLocation::from_range(source_range_for_span(
                self.source,
                parsed_block.block.comment_start_index(),
                end_block.block.comment_end_index(),
            )),
        )
        .with_placeholder_index(placeholder_index)
        .with_sample_sql(self.source[sample_start..sample_end].to_owned());

        if let Some(repeat) = self.open_repeat.as_mut() {
            repeat.item_param_usages.push(usage);
        } else {
            self.param_usages.push(usage);
        }

        self.cursor = end_block.block.comment_end_index();
        Ok(())
    }

    fn open_repeat(
        &mut self,
        parsed_block: &ParsedSqlayBlock<'a>,
        id: &str,
        separator: &str,
    ) -> core::DiagnosticResult<()> {
        debug_assert!(self.open_repeat.is_none());
        self.append_non_param_sql(parsed_block.block.comment_start_index())?;
        self.open_repeat = Some(PendingRepeat {
            id: id.to_owned(),
            separator: separator.to_owned(),
            start_index: self.analysis_sql.len(),
            start_block: parsed_block.block,
            item_param_usages: Vec::new(),
        });
        self.cursor = parsed_block.block.comment_end_index();
        Ok(())
    }

    fn close_repeat(&mut self, parsed_block: &ParsedSqlayBlock<'a>) -> core::DiagnosticResult<()> {
        self.append_non_param_sql(parsed_block.block.comment_start_index())?;
        let repeat = self
            .open_repeat
            .take()
            .expect("Repeat marker pairing is validated before replacement");
        self.repeat_usages.push(
            core::RepeatUsage::new(
                repeat.id,
                repeat.separator,
                repeat.start_index,
                self.analysis_sql.len(),
                core::SourceLocation::from_range(source_range_for_span(
                    self.source,
                    repeat.start_block.comment_start_index(),
                    parsed_block.block.comment_end_index(),
                )),
            )
            .with_item_param_usages(repeat.item_param_usages),
        );
        self.cursor = parsed_block.block.comment_end_index();
        Ok(())
    }

    fn append_non_param_sql(&mut self, end: usize) -> core::DiagnosticResult<()> {
        append_non_param_sql(self.source, self.cursor, end, &mut self.analysis_sql)
    }

    fn finish(
        mut self,
        body_end: usize,
        body_range: core::SourceRange,
    ) -> core::DiagnosticResult<InlineMarkerReplacement> {
        self.append_non_param_sql(body_end)?;
        let repeat_item_param_count = self
            .repeat_usages
            .iter()
            .map(|usage| usage.item_param_usages().len())
            .sum::<usize>();
        verify_placeholder_count(
            &self.analysis_sql,
            self.param_usages.len() + repeat_item_param_count,
            body_range,
        )?;

        Ok(InlineMarkerReplacement {
            analysis_sql: self.analysis_sql,
            param_usages: self.param_usages,
            slot_usages: self.slot_usages,
            repeat_usages: self.repeat_usages,
        })
    }
}

struct PendingRepeat<'a> {
    id: String,
    separator: String,
    start_index: usize,
    start_block: &'a SqlayBlock,
    item_param_usages: Vec<core::ParamUsage>,
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
    let placeholder_count = placeholder_count(analysis_sql);
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

/// Validates the structural constraints of inline `@sqlay` markers.
///
/// Ensures that paired inline markers are balanced without unsupported nesting,
/// that inline markers appear only inside supported source-unit bodies, and that
/// Slot and Repeat placement follows the accepted source-intake model.
pub(super) fn validate_inline_markers(
    parsed_blocks: &[ParsedSqlayBlock<'_>],
) -> core::DiagnosticResult<()> {
    let mut validator = InlineMarkerValidator::new();
    for parsed_block in parsed_blocks {
        validator.visit(parsed_block)?;
    }

    validator.finish()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InlineMarkerContext {
    OutsideSourceUnit,
    QueryBody,
    MutationBody,
    FragmentBody,
}

struct InlineMarkerValidator<'a> {
    context: InlineMarkerContext,
    open_param_block: Option<&'a SqlayBlock>,
    open_repeat_block: Option<&'a SqlayBlock>,
    open_repeat_contains_param: bool,
}

impl<'a> InlineMarkerValidator<'a> {
    const fn new() -> Self {
        Self {
            context: InlineMarkerContext::OutsideSourceUnit,
            open_param_block: None,
            open_repeat_block: None,
            open_repeat_contains_param: false,
        }
    }

    fn visit(&mut self, parsed_block: &'a ParsedSqlayBlock<'a>) -> core::DiagnosticResult<()> {
        match &parsed_block.annotation {
            SqlayAnnotation::Query(_) => self.start_source_unit(InlineMarkerContext::QueryBody),
            SqlayAnnotation::Mutation(_) => {
                self.start_source_unit(InlineMarkerContext::MutationBody)
            }
            SqlayAnnotation::Fragment(_) => {
                self.start_source_unit(InlineMarkerContext::FragmentBody)
            }
            SqlayAnnotation::Param(_) => self.visit_param(parsed_block.block),
            SqlayAnnotation::ParamEnd => self.visit_param_end(parsed_block.block),
            SqlayAnnotation::Slot(_) => self.visit_slot(parsed_block.block),
            SqlayAnnotation::Repeat(_) => self.visit_repeat(parsed_block.block),
            SqlayAnnotation::RepeatEnd => self.visit_repeat_end(parsed_block.block),
        }
    }

    fn start_source_unit(&mut self, context: InlineMarkerContext) -> core::DiagnosticResult<()> {
        if let Some(block) = self.open_param_block.take() {
            return Err(metadata_error(
                "`param` marker is missing a matching `paramEnd` marker",
                block.payload_range(),
            ));
        }
        if let Some(block) = self.open_repeat_block.take() {
            return Err(metadata_error(
                "`repeat` marker is missing a matching `repeatEnd` marker",
                block.payload_range(),
            ));
        }
        self.open_repeat_contains_param = false;
        self.context = context;
        Ok(())
    }

    fn visit_param(&mut self, block: &'a SqlayBlock) -> core::DiagnosticResult<()> {
        if self.context == InlineMarkerContext::OutsideSourceUnit {
            return Err(metadata_error(
                "`param` markers must appear inside a query, mutation, or fragment body; top-level Param markers are not supported",
                block.payload_range(),
            ));
        }
        if self.open_param_block.is_some() {
            return Err(metadata_error(
                "nested Param ranges are not supported",
                block.payload_range(),
            ));
        }
        if self.open_repeat_block.is_some() {
            self.open_repeat_contains_param = true;
        }
        self.open_param_block = Some(block);
        Ok(())
    }

    fn visit_param_end(&mut self, block: &SqlayBlock) -> core::DiagnosticResult<()> {
        if self.context == InlineMarkerContext::OutsideSourceUnit {
            return Err(metadata_error(
                "`paramEnd` markers must appear inside a query, mutation, or fragment body; top-level paramEnd markers are not supported",
                block.payload_range(),
            ));
        }
        if self.open_param_block.take().is_none() {
            return Err(metadata_error(
                "`paramEnd` marker has no matching `param` marker",
                block.payload_range(),
            ));
        }
        Ok(())
    }

    fn visit_slot(&self, block: &SqlayBlock) -> core::DiagnosticResult<()> {
        match self.context {
            InlineMarkerContext::OutsideSourceUnit => {
                return Err(metadata_error(
                    "`slot` markers must appear inside a query or mutation body; top-level Slot markers are not supported",
                    block.payload_range(),
                ));
            }
            InlineMarkerContext::FragmentBody => {
                return Err(metadata_error(
                    "slot markers inside fragments are not supported yet; define slots in query or mutation bodies",
                    block.payload_range(),
                ));
            }
            InlineMarkerContext::QueryBody | InlineMarkerContext::MutationBody => {}
        }
        if self.open_param_block.is_some() {
            return Err(metadata_error(
                "Slot markers are not supported inside Param ranges",
                block.payload_range(),
            ));
        }
        if self.open_repeat_block.is_some() {
            return Err(metadata_error(
                "Slot markers are not supported inside Repeat ranges",
                block.payload_range(),
            ));
        }
        Ok(())
    }

    fn visit_repeat(&mut self, block: &'a SqlayBlock) -> core::DiagnosticResult<()> {
        if self.context == InlineMarkerContext::OutsideSourceUnit {
            return Err(metadata_error(
                "`repeat` markers must appear inside a query, mutation, or fragment body; top-level Repeat markers are not supported",
                block.payload_range(),
            ));
        }
        if self.open_param_block.is_some() {
            return Err(metadata_error(
                "Repeat markers are not supported inside Param ranges",
                block.payload_range(),
            ));
        }
        if self.open_repeat_block.is_some() {
            return Err(metadata_error(
                "nested Repeat ranges are not supported",
                block.payload_range(),
            ));
        }
        self.open_repeat_block = Some(block);
        self.open_repeat_contains_param = false;
        Ok(())
    }

    fn visit_repeat_end(&mut self, block: &SqlayBlock) -> core::DiagnosticResult<()> {
        if self.context == InlineMarkerContext::OutsideSourceUnit {
            return Err(metadata_error(
                "`repeatEnd` markers must appear inside a query, mutation, or fragment body; top-level repeatEnd markers are not supported",
                block.payload_range(),
            ));
        }
        if self.open_param_block.is_some() {
            return Err(metadata_error(
                "Repeat markers are not supported inside Param ranges",
                block.payload_range(),
            ));
        }
        if self.open_repeat_block.take().is_none() {
            return Err(metadata_error(
                "`repeatEnd` marker has no matching `repeat` marker",
                block.payload_range(),
            ));
        }
        if !self.open_repeat_contains_param {
            return Err(metadata_error(
                "Repeat ranges must contain at least one Param marker",
                block.payload_range(),
            ));
        }
        self.open_repeat_contains_param = false;
        Ok(())
    }

    fn finish(self) -> core::DiagnosticResult<()> {
        if let Some(block) = self.open_param_block {
            return Err(metadata_error(
                "`param` marker is missing a matching `paramEnd` marker",
                block.payload_range(),
            ));
        }
        if let Some(block) = self.open_repeat_block {
            return Err(metadata_error(
                "`repeat` marker is missing a matching `repeatEnd` marker",
                block.payload_range(),
            ));
        }
        Ok(())
    }
}
