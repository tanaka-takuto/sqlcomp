use sqlay_core as core;

use super::super::diagnostics::{
    mutation_param_placeholder_index, mutation_param_usage_error, param_usage_error,
    query_param_placeholder_index,
};
use super::super::slot_variants::{
    ExpandedParamBuffers, ExpandedParamOccurrence, ExpandedParamScope,
};
use super::owner::RepeatOwner;

pub(in crate::compile) fn push_query_segment_with_repeats(
    query: &core::RawQuery,
    segment_start: usize,
    limit: usize,
    query_param_cursor: &mut usize,
    query_repeat_cursor: &mut usize,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    let mut cursor = segment_start;

    while let Some(repeat) = query.repeat_usages().get(*query_repeat_cursor) {
        if repeat.start_index() >= limit {
            break;
        }
        if repeat.start_index() < cursor || repeat.end_index() > limit {
            return Err(RepeatOwner::Query(query).repeat_usage_error(
                repeat,
                format!(
                    "Repeat `{}` in query `{}` crosses a Slot insertion boundary; Repeat ranges must stay within one validation SQL segment",
                    repeat.id(),
                    query.metadata().id()
                ),
            ));
        }

        let segment_output_start = analysis_sql.len();
        analysis_sql.push_str(&query.analysis_sql()[cursor..repeat.start_index()]);
        push_query_params_before_index(
            query,
            cursor,
            segment_output_start,
            repeat.start_index(),
            query_param_cursor,
            params,
        )?;
        push_repeat_representative(RepeatOwner::Query(query), repeat, analysis_sql, params)?;

        cursor = repeat.end_index();
        *query_repeat_cursor += 1;
    }

    let segment_output_start = analysis_sql.len();
    analysis_sql.push_str(&query.analysis_sql()[cursor..limit]);
    push_query_params_before_index(
        query,
        cursor,
        segment_output_start,
        limit,
        query_param_cursor,
        params,
    )
}

pub(in crate::compile) fn push_mutation_segment_with_repeats(
    mutation: &core::RawMutation,
    segment_start: usize,
    limit: usize,
    mutation_param_cursor: &mut usize,
    mutation_repeat_cursor: &mut usize,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    let mut cursor = segment_start;

    while let Some(repeat) = mutation.repeat_usages().get(*mutation_repeat_cursor) {
        if repeat.start_index() >= limit {
            break;
        }
        if repeat.start_index() < cursor || repeat.end_index() > limit {
            return Err(RepeatOwner::Mutation(mutation).repeat_usage_error(
                repeat,
                format!(
                    "Repeat `{}` in mutation `{}` crosses a Slot insertion boundary; Repeat ranges must stay within one validation SQL segment",
                    repeat.id(),
                    mutation.metadata().id()
                ),
            ));
        }

        let segment_output_start = analysis_sql.len();
        analysis_sql.push_str(&mutation.analysis_sql()[cursor..repeat.start_index()]);
        push_mutation_params_before_index(
            mutation,
            cursor,
            segment_output_start,
            repeat.start_index(),
            mutation_param_cursor,
            params,
        )?;
        push_repeat_representative(
            RepeatOwner::Mutation(mutation),
            repeat,
            analysis_sql,
            params,
        )?;

        cursor = repeat.end_index();
        *mutation_repeat_cursor += 1;
    }

    let segment_output_start = analysis_sql.len();
    analysis_sql.push_str(&mutation.analysis_sql()[cursor..limit]);
    push_mutation_params_before_index(
        mutation,
        cursor,
        segment_output_start,
        limit,
        mutation_param_cursor,
        params,
    )
}

pub(in crate::compile) fn push_fragment_segment_with_repeats(
    fragment: &core::RawFragment,
    query: &core::RawQuery,
    slot_usage: &core::SlotUsage,
    slot_occurrence_index: usize,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    push_fragment_segment(
        RepeatOwner::FragmentQuery {
            query,
            slot_usage,
            slot_occurrence_index,
            fragment,
        },
        fragment,
        analysis_sql,
        params,
    )
}

pub(in crate::compile) fn push_mutation_fragment_segment_with_repeats(
    fragment: &core::RawFragment,
    mutation: &core::RawMutation,
    slot_usage: &core::SlotUsage,
    slot_occurrence_index: usize,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    push_fragment_segment(
        RepeatOwner::FragmentMutation {
            mutation,
            slot_usage,
            slot_occurrence_index,
            fragment,
        },
        fragment,
        analysis_sql,
        params,
    )
}

fn push_fragment_segment(
    owner: RepeatOwner<'_>,
    fragment: &core::RawFragment,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    let mut cursor = 0;
    let mut fragment_param_cursor = 0;

    for repeat in fragment.repeat_usages() {
        if repeat.start_index() < cursor || repeat.end_index() > fragment.analysis_sql().len() {
            return Err(owner.repeat_usage_error(
                repeat,
                format!(
                    "Repeat `{}` in Fragment `{}` selected by Slot `{}` in {} `{}` has invalid analysis SQL range {}..{}",
                    repeat.id(),
                    fragment.metadata().id(),
                    owner.slot_id(),
                    owner.source_kind(),
                    owner.source_id(),
                    repeat.start_index(),
                    repeat.end_index()
                ),
            ));
        }

        let segment_output_start = analysis_sql.len();
        analysis_sql.push_str(&fragment.analysis_sql()[cursor..repeat.start_index()]);
        push_fragment_params_before_index(
            owner,
            fragment,
            cursor,
            segment_output_start,
            repeat.start_index(),
            &mut fragment_param_cursor,
            params,
        )?;
        push_repeat_representative(owner, repeat, analysis_sql, params)?;

        cursor = repeat.end_index();
    }

    let segment_output_start = analysis_sql.len();
    analysis_sql.push_str(&fragment.analysis_sql()[cursor..]);
    push_fragment_params_before_index(
        owner,
        fragment,
        cursor,
        segment_output_start,
        fragment.analysis_sql().len(),
        &mut fragment_param_cursor,
        params,
    )
}

fn push_query_params_before_index(
    query: &core::RawQuery,
    segment_start: usize,
    segment_output_start: usize,
    limit: usize,
    query_param_cursor: &mut usize,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    while let Some(usage) = query.param_usages().get(*query_param_cursor) {
        let placeholder_index = query_param_placeholder_index(query, usage)?;
        if placeholder_index >= limit {
            break;
        }
        if placeholder_index < segment_start {
            return Err(param_usage_error(
                query,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Slot expansion cursor {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.usages.push(
            usage
                .clone()
                .with_placeholder_index(segment_output_start + placeholder_index - segment_start),
        );
        params.scopes.push(ExpandedParamScope::QueryDirect);
        params
            .occurrences
            .push(ExpandedParamOccurrence::QueryDirect);
        *query_param_cursor += 1;
    }

    Ok(())
}

fn push_mutation_params_before_index(
    mutation: &core::RawMutation,
    segment_start: usize,
    segment_output_start: usize,
    limit: usize,
    mutation_param_cursor: &mut usize,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    while let Some(usage) = mutation.param_usages().get(*mutation_param_cursor) {
        let placeholder_index = mutation_param_placeholder_index(mutation, usage)?;
        if placeholder_index >= limit {
            break;
        }
        if placeholder_index < segment_start {
            return Err(mutation_param_usage_error(
                mutation,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Slot expansion cursor {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.usages.push(
            usage
                .clone()
                .with_placeholder_index(segment_output_start + placeholder_index - segment_start),
        );
        params.scopes.push(ExpandedParamScope::QueryDirect);
        params
            .occurrences
            .push(ExpandedParamOccurrence::QueryDirect);
        *mutation_param_cursor += 1;
    }

    Ok(())
}

fn push_fragment_params_before_index(
    owner: RepeatOwner<'_>,
    fragment: &core::RawFragment,
    segment_start: usize,
    segment_output_start: usize,
    limit: usize,
    fragment_param_cursor: &mut usize,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    while let Some(usage) = fragment.param_usages().get(*fragment_param_cursor) {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(owner.fragment_param_error(
                usage,
                format!(
                    "Param `{}` in fragment `{}` is missing placeholder position metadata",
                    usage.id(),
                    fragment.metadata().id()
                ),
            ));
        };
        if placeholder_index >= limit {
            break;
        }
        if placeholder_index < segment_start {
            return Err(owner.fragment_param_error(
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Fragment Repeat expansion cursor {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.usages.push(
            usage
                .clone()
                .with_placeholder_index(segment_output_start + placeholder_index - segment_start),
        );
        owner.push_fragment_param_context(params);
        *fragment_param_cursor += 1;
    }

    Ok(())
}

fn push_repeat_representative(
    owner: RepeatOwner<'_>,
    repeat: &core::RepeatUsage,
    analysis_sql: &mut String,
    params: &mut ExpandedParamBuffers,
) -> core::DiagnosticResult<()> {
    for representative_item_index in 0..2 {
        if representative_item_index > 0 {
            analysis_sql.push_str(repeat.separator());
        }

        let item_output_start = analysis_sql.len();
        analysis_sql.push_str(&owner.analysis_sql()[repeat.start_index()..repeat.end_index()]);
        for usage in repeat.item_param_usages() {
            let placeholder_index = owner.repeat_param_placeholder_index(repeat, usage)?;
            params.usages.push(usage.clone().with_placeholder_index(
                item_output_start + placeholder_index - repeat.start_index(),
            ));
            owner.push_repeat_param_context(params, repeat, representative_item_index + 1);
        }
    }

    Ok(())
}
