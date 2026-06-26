use sqlay_core as core;

use crate::source_fs::inline_markers::{
    reject_fragment_statement_separator, replace_inline_markers, validate_inline_markers,
};
use crate::source_fs::metadata::{ParsedSqlayBlock, SqlayAnnotation, parse_sqlay_annotation};
use crate::source_fs::scanner::{SqlayBlockScan, scan_sqlay_blocks, source_range_for_sql_body};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SourceUnits {
    queries: Vec<core::RawQuery>,
    mutations: Vec<core::RawMutation>,
    fragments: Vec<core::RawFragment>,
    units: Vec<core::RawSourceUnit>,
}

impl SourceUnits {
    const fn new(
        queries: Vec<core::RawQuery>,
        mutations: Vec<core::RawMutation>,
        fragments: Vec<core::RawFragment>,
        units: Vec<core::RawSourceUnit>,
    ) -> Self {
        Self {
            queries,
            mutations,
            fragments,
            units,
        }
    }

    #[cfg(test)]
    pub(super) fn queries(&self) -> &[core::RawQuery] {
        &self.queries
    }

    #[cfg(test)]
    pub(super) fn mutations(&self) -> &[core::RawMutation] {
        &self.mutations
    }

    #[cfg(test)]
    pub(super) fn fragments(&self) -> &[core::RawFragment] {
        &self.fragments
    }

    #[cfg(test)]
    pub(super) fn source_units(&self) -> &[core::RawSourceUnit] {
        &self.units
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        Vec<core::RawQuery>,
        Vec<core::RawMutation>,
        Vec<core::RawFragment>,
        Vec<core::RawSourceUnit>,
    ) {
        (self.queries, self.mutations, self.fragments, self.units)
    }
}

/// Split SQL source text into raw query blocks.
///
/// # Errors
///
/// Returns diagnostics when sqlay block scanning fails or any query metadata
/// payload is invalid.
pub fn split_sqlay_query_blocks(source: &str) -> core::DiagnosticResult<Vec<core::RawQuery>> {
    split_sqlay_source_units(source).map(|source_units| source_units.queries)
}

pub(super) fn split_sqlay_source_units(source: &str) -> core::DiagnosticResult<SourceUnits> {
    let scan = scan_sqlay_blocks(source)?;
    split_sqlay_source_units_from_scan(source, &scan)
}

pub(super) fn split_sqlay_source_units_from_scan(
    source: &str,
    scan: &SqlayBlockScan,
) -> core::DiagnosticResult<SourceUnits> {
    let blocks = scan.blocks();
    let mut parsed_blocks = Vec::with_capacity(blocks.len());

    for block in blocks {
        parsed_blocks.push(ParsedSqlayBlock {
            block,
            annotation: parse_sqlay_annotation(block)?,
        });
    }

    validate_inline_markers(&parsed_blocks)?;

    let source_unit_indexes = parsed_blocks
        .iter()
        .enumerate()
        .filter_map(|(index, parsed_block)| is_global_source_unit(parsed_block).then_some(index))
        .collect::<Vec<_>>();
    let mut queries = Vec::new();
    let mut mutations = Vec::new();
    let mut fragments = Vec::new();
    let mut source_units = Vec::new();

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
            SqlayAnnotation::Query(metadata) => {
                let replacement =
                    replace_inline_markers(source, body_start, body_end, &parsed_blocks)?;

                let query = core::RawQuery::new(metadata.clone(), sql)
                    .with_analysis_sql(replacement.analysis_sql)
                    .with_param_usages(replacement.param_usages)
                    .with_slot_usages(replacement.slot_usages)
                    .with_source_location(location);
                source_units.push(core::RawSourceUnit::Query(query.clone()));
                queries.push(query);
            }
            SqlayAnnotation::Mutation(metadata) => {
                let replacement =
                    replace_inline_markers(source, body_start, body_end, &parsed_blocks)?;

                let mutation = core::RawMutation::new(metadata.clone(), sql)
                    .with_analysis_sql(replacement.analysis_sql)
                    .with_param_usages(replacement.param_usages)
                    .with_slot_usages(replacement.slot_usages)
                    .with_source_location(location);
                source_units.push(core::RawSourceUnit::Mutation(mutation.clone()));
                mutations.push(mutation);
            }
            SqlayAnnotation::Fragment(metadata) => {
                reject_fragment_statement_separator(source, body_start, body_end)?;
                let replacement =
                    replace_inline_markers(source, body_start, body_end, &parsed_blocks)?;
                debug_assert!(replacement.slot_usages.is_empty());
                let fragment = core::RawFragment::new(metadata.clone(), sql)
                    .with_analysis_sql(replacement.analysis_sql)
                    .with_param_usages(replacement.param_usages)
                    .with_source_location(location);
                source_units.push(core::RawSourceUnit::Fragment(fragment.clone()));
                fragments.push(fragment);
            }
            SqlayAnnotation::Param(_) | SqlayAnnotation::ParamEnd | SqlayAnnotation::Slot(_) => {
                unreachable!("source unit indexes only point at global annotations");
            }
        }
    }

    Ok(SourceUnits::new(
        queries,
        mutations,
        fragments,
        source_units,
    ))
}

const fn is_global_source_unit(parsed_block: &ParsedSqlayBlock<'_>) -> bool {
    matches!(
        parsed_block.annotation,
        SqlayAnnotation::Query(_) | SqlayAnnotation::Mutation(_) | SqlayAnnotation::Fragment(_)
    )
}
