use sqlay_core as core;

use super::slot_variants::{ExpandedParamBuffers, ExpandedParamOccurrence, ExpandedParamScope};

mod owner;
mod segments;

pub(super) use segments::{
    push_fragment_segment_with_repeats, push_mutation_fragment_segment_with_repeats,
    push_mutation_segment_with_repeats, push_query_segment_with_repeats,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RepeatExpansionQuery {
    pub(super) query: core::RawQuery,
    pub(super) param_scopes: Vec<ExpandedParamScope>,
    pub(super) param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RepeatExpansionMutation {
    pub(super) mutation: core::RawMutation,
    pub(super) param_scopes: Vec<ExpandedParamScope>,
    pub(super) param_occurrences: Vec<ExpandedParamOccurrence>,
}

pub(super) fn build_representative_query(
    query: &core::RawQuery,
) -> core::DiagnosticResult<RepeatExpansionQuery> {
    let mut analysis_sql = String::with_capacity(query.analysis_sql().len());
    let mut query_param_cursor = 0;
    let mut query_repeat_cursor = 0;
    let mut params = ExpandedParamBuffers::default();

    push_query_segment_with_repeats(
        query,
        0,
        query.analysis_sql().len(),
        &mut query_param_cursor,
        &mut query_repeat_cursor,
        &mut analysis_sql,
        &mut params,
    )?;

    let mut expanded_query = core::RawQuery::new(query.metadata().clone(), query.sql().to_owned())
        .with_analysis_sql(analysis_sql)
        .with_param_usages(params.usages);

    if let Some(source_path) = query.source_path() {
        expanded_query = expanded_query.with_source_path(source_path.to_path_buf());
    }
    if let Some(source_location) = query.source_location() {
        expanded_query = expanded_query.with_source_location(source_location.clone());
    }

    Ok(RepeatExpansionQuery {
        query: expanded_query,
        param_scopes: params.scopes,
        param_occurrences: params.occurrences,
    })
}

pub(super) fn build_representative_mutation(
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<RepeatExpansionMutation> {
    let mut analysis_sql = String::with_capacity(mutation.analysis_sql().len());
    let mut mutation_param_cursor = 0;
    let mut mutation_repeat_cursor = 0;
    let mut params = ExpandedParamBuffers::default();

    push_mutation_segment_with_repeats(
        mutation,
        0,
        mutation.analysis_sql().len(),
        &mut mutation_param_cursor,
        &mut mutation_repeat_cursor,
        &mut analysis_sql,
        &mut params,
    )?;

    let mut expanded_mutation =
        core::RawMutation::new(mutation.metadata().clone(), mutation.sql().to_owned())
            .with_analysis_sql(analysis_sql)
            .with_param_usages(params.usages);

    if let Some(source_path) = mutation.source_path() {
        expanded_mutation = expanded_mutation.with_source_path(source_path.to_path_buf());
    }
    if let Some(source_location) = mutation.source_location() {
        expanded_mutation = expanded_mutation.with_source_location(source_location.clone());
    }

    Ok(RepeatExpansionMutation {
        mutation: expanded_mutation,
        param_scopes: params.scopes,
        param_occurrences: params.occurrences,
    })
}
