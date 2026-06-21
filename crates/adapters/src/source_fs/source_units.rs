use sqlcomp_core as core;

use crate::source_fs::inline_markers::{
    reject_fragment_statement_separator, replace_inline_markers, validate_inline_markers,
};
use crate::source_fs::metadata::{ParsedSqlcompBlock, SqlcompAnnotation, parse_sqlcomp_annotation};
use crate::source_fs::scanner::{SqlcompBlockScan, scan_sqlcomp_blocks, source_range_for_sql_body};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SourceUnits {
    queries: Vec<core::RawQuery>,
    fragments: Vec<core::RawFragment>,
}

impl SourceUnits {
    const fn new(queries: Vec<core::RawQuery>, fragments: Vec<core::RawFragment>) -> Self {
        Self { queries, fragments }
    }

    #[cfg(test)]
    pub(super) fn queries(&self) -> &[core::RawQuery] {
        &self.queries
    }

    #[cfg(test)]
    pub(super) fn fragments(&self) -> &[core::RawFragment] {
        &self.fragments
    }

    pub(super) fn into_parts(self) -> (Vec<core::RawQuery>, Vec<core::RawFragment>) {
        (self.queries, self.fragments)
    }
}

/// Split SQL source text into raw query blocks.
///
/// # Errors
///
/// Returns diagnostics when sqlcomp block scanning fails or any query metadata
/// payload is invalid.
pub fn split_sqlcomp_query_blocks(source: &str) -> core::DiagnosticResult<Vec<core::RawQuery>> {
    split_sqlcomp_source_units(source).map(|source_units| source_units.queries)
}

pub(super) fn split_sqlcomp_source_units(source: &str) -> core::DiagnosticResult<SourceUnits> {
    let scan = scan_sqlcomp_blocks(source)?;
    split_sqlcomp_source_units_from_scan(source, &scan)
}

pub(super) fn split_sqlcomp_source_units_from_scan(
    source: &str,
    scan: &SqlcompBlockScan,
) -> core::DiagnosticResult<SourceUnits> {
    let blocks = scan.blocks();
    let mut parsed_blocks = Vec::with_capacity(blocks.len());

    for block in blocks {
        parsed_blocks.push(ParsedSqlcompBlock {
            block,
            annotation: parse_sqlcomp_annotation(block)?,
        });
    }

    validate_inline_markers(&parsed_blocks)?;

    let source_unit_indexes = parsed_blocks
        .iter()
        .enumerate()
        .filter_map(|(index, parsed_block)| is_global_source_unit(parsed_block).then_some(index))
        .collect::<Vec<_>>();
    let mut queries = Vec::new();
    let mut fragments = Vec::new();

    for (source_unit_position, parsed_index) in source_unit_indexes.iter().copied().enumerate() {
        let parsed_block = &parsed_blocks[parsed_index];
        let body_start = parsed_block.block.comment_end_index();
        let body_end = source_unit_indexes.get(source_unit_position + 1).map_or(
            source.len(),
            |next_source_unit_index| {
                parsed_blocks[*next_source_unit_index]
                    .block
                    .comment_start_index()
            },
        );
        let sql = source[body_start..body_end].to_owned();
        let location = core::SourceLocation::from_range(source_range_for_sql_body(
            source, body_start, body_end,
        ));

        match &parsed_block.annotation {
            SqlcompAnnotation::Query(metadata) => {
                let replacement =
                    replace_inline_markers(source, body_start, body_end, &parsed_blocks)?;

                queries.push(
                    core::RawQuery::new(metadata.clone(), sql)
                        .with_analysis_sql(replacement.analysis_sql)
                        .with_param_usages(replacement.param_usages)
                        .with_slot_usages(replacement.slot_usages)
                        .with_source_location(location),
                );
            }
            SqlcompAnnotation::Fragment(metadata) => {
                reject_fragment_statement_separator(source, body_start, body_end)?;
                let replacement =
                    replace_inline_markers(source, body_start, body_end, &parsed_blocks)?;
                debug_assert!(replacement.slot_usages.is_empty());
                fragments.push(
                    core::RawFragment::new(metadata.clone(), sql)
                        .with_analysis_sql(replacement.analysis_sql)
                        .with_param_usages(replacement.param_usages)
                        .with_source_location(location),
                );
            }
            SqlcompAnnotation::Param(_)
            | SqlcompAnnotation::ParamEnd
            | SqlcompAnnotation::Slot(_) => {
                unreachable!("source unit indexes only point at global annotations");
            }
        }
    }

    Ok(SourceUnits::new(queries, fragments))
}

const fn is_global_source_unit(parsed_block: &ParsedSqlcompBlock<'_>) -> bool {
    matches!(
        parsed_block.annotation,
        SqlcompAnnotation::Query(_) | SqlcompAnnotation::Fragment(_)
    )
}
