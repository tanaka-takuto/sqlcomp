use sqlay_core as core;

use super::super::diagnostics::{mutation_param_usage_error, param_usage_error, query_error};
use super::super::param_validation::ScopedParamBinding;
use super::super::slot_variants::ExpandedParamScope;
use super::{
    RepeatBindingScope, compiled_base_segment, compiled_mutation_base_segment,
    compiled_mutation_param_binding, compiled_param_binding,
};

pub(super) fn compiled_query_body(
    query: &core::RawQuery,
    segment_start: usize,
    segment_end: usize,
    query_param_cursor: &mut usize,
    query_repeat_cursor: &mut usize,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlBody> {
    let mut base_segments = Vec::new();
    let mut repeat_occurrences = Vec::new();
    let mut cursor = segment_start;

    while let Some(repeat) = query.repeat_usages().get(*query_repeat_cursor) {
        if repeat.start_index() >= segment_end {
            break;
        }
        if repeat.start_index() < cursor || repeat.end_index() > segment_end {
            return Err(query_error(
                query,
                format!(
                    "Repeat `{}` in query `{}` crosses a Slot Core IR segment boundary",
                    repeat.id(),
                    query.metadata().id()
                ),
            ));
        }

        base_segments.push(compiled_base_segment(
            query,
            cursor,
            repeat.start_index(),
            query_param_cursor,
            scoped_param_bindings,
        )?);
        repeat_occurrences.push(compiled_query_repeat_occurrence(
            query,
            repeat,
            &RepeatBindingScope::Builder,
            scoped_param_bindings,
        )?);
        cursor = repeat.end_index();
        *query_repeat_cursor += 1;
    }

    base_segments.push(compiled_base_segment(
        query,
        cursor,
        segment_end,
        query_param_cursor,
        scoped_param_bindings,
    )?);

    Ok(core::CompiledSqlBody::new(
        base_segments,
        repeat_occurrences,
    ))
}

pub(super) fn compiled_mutation_body(
    mutation: &core::RawMutation,
    segment_start: usize,
    segment_end: usize,
    mutation_param_cursor: &mut usize,
    mutation_repeat_cursor: &mut usize,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlBody> {
    let mut base_segments = Vec::new();
    let mut repeat_occurrences = Vec::new();
    let mut cursor = segment_start;

    while let Some(repeat) = mutation.repeat_usages().get(*mutation_repeat_cursor) {
        if repeat.start_index() >= segment_end {
            break;
        }
        if repeat.start_index() < cursor || repeat.end_index() > segment_end {
            return Err(super::super::diagnostics::mutation_error(
                mutation,
                format!(
                    "Repeat `{}` in mutation `{}` crosses a Slot Core IR segment boundary",
                    repeat.id(),
                    mutation.metadata().id()
                ),
            ));
        }

        base_segments.push(compiled_mutation_base_segment(
            mutation,
            cursor,
            repeat.start_index(),
            mutation_param_cursor,
            scoped_param_bindings,
        )?);
        repeat_occurrences.push(compiled_mutation_repeat_occurrence(
            mutation,
            repeat,
            &RepeatBindingScope::Builder,
            scoped_param_bindings,
        )?);
        cursor = repeat.end_index();
        *mutation_repeat_cursor += 1;
    }

    base_segments.push(compiled_mutation_base_segment(
        mutation,
        cursor,
        segment_end,
        mutation_param_cursor,
        scoped_param_bindings,
    )?);

    Ok(core::CompiledSqlBody::new(
        base_segments,
        repeat_occurrences,
    ))
}

pub(super) fn compiled_query_fragment_body(
    query: &core::RawQuery,
    fragment: &core::RawFragment,
    fragment_scope: &ExpandedParamScope,
    repeat_scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlBody> {
    let mut base_segments = Vec::new();
    let mut repeat_occurrences = Vec::new();
    let mut cursor = 0;
    let mut fragment_param_cursor = 0;

    for repeat in fragment.repeat_usages() {
        if repeat.start_index() < cursor || repeat.end_index() > fragment.analysis_sql().len() {
            return Err(query_error(
                query,
                format!(
                    "Repeat `{}` in Fragment `{}` has invalid Core IR range {}..{}",
                    repeat.id(),
                    fragment.metadata().id(),
                    repeat.start_index(),
                    repeat.end_index()
                ),
            ));
        }

        base_segments.push(compiled_query_fragment_base_segment(
            query,
            fragment,
            cursor,
            repeat.start_index(),
            &mut fragment_param_cursor,
            fragment_scope,
            scoped_param_bindings,
        )?);
        repeat_occurrences.push(compiled_query_fragment_repeat_occurrence(
            query,
            fragment,
            repeat,
            repeat_scope,
            scoped_param_bindings,
        )?);
        cursor = repeat.end_index();
    }

    base_segments.push(compiled_query_fragment_base_segment(
        query,
        fragment,
        cursor,
        fragment.analysis_sql().len(),
        &mut fragment_param_cursor,
        fragment_scope,
        scoped_param_bindings,
    )?);

    Ok(core::CompiledSqlBody::new(
        base_segments,
        repeat_occurrences,
    ))
}

pub(super) fn compiled_mutation_fragment_body(
    mutation: &core::RawMutation,
    fragment: &core::RawFragment,
    fragment_scope: &ExpandedParamScope,
    repeat_scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlBody> {
    let mut base_segments = Vec::new();
    let mut repeat_occurrences = Vec::new();
    let mut cursor = 0;
    let mut fragment_param_cursor = 0;

    for repeat in fragment.repeat_usages() {
        if repeat.start_index() < cursor || repeat.end_index() > fragment.analysis_sql().len() {
            return Err(super::super::diagnostics::mutation_error(
                mutation,
                format!(
                    "Repeat `{}` in Fragment `{}` has invalid Core IR range {}..{}",
                    repeat.id(),
                    fragment.metadata().id(),
                    repeat.start_index(),
                    repeat.end_index()
                ),
            ));
        }

        base_segments.push(compiled_mutation_fragment_base_segment(
            mutation,
            fragment,
            cursor,
            repeat.start_index(),
            &mut fragment_param_cursor,
            fragment_scope,
            scoped_param_bindings,
        )?);
        repeat_occurrences.push(compiled_mutation_fragment_repeat_occurrence(
            mutation,
            fragment,
            repeat,
            repeat_scope,
            scoped_param_bindings,
        )?);
        cursor = repeat.end_index();
    }

    base_segments.push(compiled_mutation_fragment_base_segment(
        mutation,
        fragment,
        cursor,
        fragment.analysis_sql().len(),
        &mut fragment_param_cursor,
        fragment_scope,
        scoped_param_bindings,
    )?);

    Ok(core::CompiledSqlBody::new(
        base_segments,
        repeat_occurrences,
    ))
}

fn compiled_query_fragment_base_segment(
    query: &core::RawQuery,
    fragment: &core::RawFragment,
    segment_start: usize,
    segment_end: usize,
    fragment_param_cursor: &mut usize,
    scope: &ExpandedParamScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlSegment> {
    let Some(sql) = fragment.analysis_sql().get(segment_start..segment_end) else {
        return Err(query_error(
            query,
            format!(
                "invalid Fragment `{}` SQL segment range {segment_start}..{segment_end} while compiling Repeat Core IR",
                fragment.metadata().id()
            ),
        ));
    };
    let mut params = Vec::new();

    while let Some(usage) = fragment.param_usages().get(*fragment_param_cursor) {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(param_usage_error(
                query,
                usage,
                format!(
                    "Param `{}` in Fragment `{}` is missing placeholder position metadata",
                    usage.id(),
                    fragment.metadata().id()
                ),
            ));
        };
        if placeholder_index >= segment_end {
            break;
        }
        if placeholder_index < segment_start {
            return Err(param_usage_error(
                query,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Fragment Core IR segment start {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.push(compiled_param_binding(
            query,
            usage,
            scope,
            scoped_param_bindings,
        )?);
        *fragment_param_cursor += 1;
    }

    Ok(core::CompiledSqlSegment::new(sql.to_owned(), params))
}

fn compiled_mutation_fragment_base_segment(
    mutation: &core::RawMutation,
    fragment: &core::RawFragment,
    segment_start: usize,
    segment_end: usize,
    fragment_param_cursor: &mut usize,
    scope: &ExpandedParamScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlSegment> {
    let Some(sql) = fragment.analysis_sql().get(segment_start..segment_end) else {
        return Err(super::super::diagnostics::mutation_error(
            mutation,
            format!(
                "invalid Fragment `{}` SQL segment range {segment_start}..{segment_end} while compiling Repeat Core IR",
                fragment.metadata().id()
            ),
        ));
    };
    let mut params = Vec::new();

    while let Some(usage) = fragment.param_usages().get(*fragment_param_cursor) {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(mutation_param_usage_error(
                mutation,
                usage,
                format!(
                    "Param `{}` in Fragment `{}` is missing placeholder position metadata",
                    usage.id(),
                    fragment.metadata().id()
                ),
            ));
        };
        if placeholder_index >= segment_end {
            break;
        }
        if placeholder_index < segment_start {
            return Err(mutation_param_usage_error(
                mutation,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Fragment Core IR segment start {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.push(compiled_mutation_param_binding(
            mutation,
            usage,
            scope,
            scoped_param_bindings,
        )?);
        *fragment_param_cursor += 1;
    }

    Ok(core::CompiledSqlSegment::new(sql.to_owned(), params))
}

fn compiled_query_repeat_occurrence(
    query: &core::RawQuery,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledRepeatOccurrence> {
    let Some(item_sql) = query
        .analysis_sql()
        .get(repeat.start_index()..repeat.end_index())
    else {
        return Err(query_error(
            query,
            format!(
                "invalid Repeat `{}` item SQL range {}..{} while compiling Repeat Core IR",
                repeat.id(),
                repeat.start_index(),
                repeat.end_index()
            ),
        ));
    };
    let params = compiled_query_repeat_params(query, repeat, scope, scoped_param_bindings)?;

    Ok(core::CompiledRepeatOccurrence::new(
        repeat.id().to_owned(),
        repeat.separator().to_owned(),
        core::CompiledSqlSegment::new(item_sql.to_owned(), params),
    ))
}

fn compiled_mutation_repeat_occurrence(
    mutation: &core::RawMutation,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledRepeatOccurrence> {
    let Some(item_sql) = mutation
        .analysis_sql()
        .get(repeat.start_index()..repeat.end_index())
    else {
        return Err(super::super::diagnostics::mutation_error(
            mutation,
            format!(
                "invalid Repeat `{}` item SQL range {}..{} while compiling Repeat Core IR",
                repeat.id(),
                repeat.start_index(),
                repeat.end_index()
            ),
        ));
    };
    let params = compiled_mutation_repeat_params(mutation, repeat, scope, scoped_param_bindings)?;

    Ok(core::CompiledRepeatOccurrence::new(
        repeat.id().to_owned(),
        repeat.separator().to_owned(),
        core::CompiledSqlSegment::new(item_sql.to_owned(), params),
    ))
}

fn compiled_query_fragment_repeat_occurrence(
    query: &core::RawQuery,
    fragment: &core::RawFragment,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledRepeatOccurrence> {
    let Some(item_sql) = fragment
        .analysis_sql()
        .get(repeat.start_index()..repeat.end_index())
    else {
        return Err(query_error(
            query,
            format!(
                "invalid Repeat `{}` item SQL range {}..{} in Fragment `{}` while compiling Repeat Core IR",
                repeat.id(),
                repeat.start_index(),
                repeat.end_index(),
                fragment.metadata().id()
            ),
        ));
    };
    let params = compiled_query_repeat_params(query, repeat, scope, scoped_param_bindings)?;

    Ok(core::CompiledRepeatOccurrence::new(
        repeat.id().to_owned(),
        repeat.separator().to_owned(),
        core::CompiledSqlSegment::new(item_sql.to_owned(), params),
    ))
}

fn compiled_mutation_fragment_repeat_occurrence(
    mutation: &core::RawMutation,
    fragment: &core::RawFragment,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledRepeatOccurrence> {
    let Some(item_sql) = fragment
        .analysis_sql()
        .get(repeat.start_index()..repeat.end_index())
    else {
        return Err(super::super::diagnostics::mutation_error(
            mutation,
            format!(
                "invalid Repeat `{}` item SQL range {}..{} in Fragment `{}` while compiling Repeat Core IR",
                repeat.id(),
                repeat.start_index(),
                repeat.end_index(),
                fragment.metadata().id()
            ),
        ));
    };
    let params = compiled_mutation_repeat_params(mutation, repeat, scope, scoped_param_bindings)?;

    Ok(core::CompiledRepeatOccurrence::new(
        repeat.id().to_owned(),
        repeat.separator().to_owned(),
        core::CompiledSqlSegment::new(item_sql.to_owned(), params),
    ))
}

fn compiled_query_repeat_params(
    query: &core::RawQuery,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::ParamBinding>> {
    let mut item_params = repeat
        .item_param_usages()
        .iter()
        .map(|usage| {
            Ok((
                query_repeat_param_placeholder_index(query, repeat, usage)?,
                usage,
            ))
        })
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    item_params.sort_by_key(|(placeholder_index, _)| *placeholder_index);
    let param_scope = scope.expanded_scope(repeat.id());
    item_params
        .into_iter()
        .map(|(_, usage)| compiled_param_binding(query, usage, &param_scope, scoped_param_bindings))
        .collect()
}

fn compiled_mutation_repeat_params(
    mutation: &core::RawMutation,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::ParamBinding>> {
    let mut item_params = repeat
        .item_param_usages()
        .iter()
        .map(|usage| {
            Ok((
                mutation_repeat_param_placeholder_index(mutation, repeat, usage)?,
                usage,
            ))
        })
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    item_params.sort_by_key(|(placeholder_index, _)| *placeholder_index);
    let param_scope = scope.expanded_scope(repeat.id());
    item_params
        .into_iter()
        .map(|(_, usage)| {
            compiled_mutation_param_binding(mutation, usage, &param_scope, scoped_param_bindings)
        })
        .collect()
}

fn query_repeat_param_placeholder_index(
    query: &core::RawQuery,
    repeat: &core::RepeatUsage,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<usize> {
    let Some(placeholder_index) = usage.placeholder_index() else {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` in Repeat `{}` is missing placeholder position metadata",
                usage.id(),
                repeat.id()
            ),
        ));
    };
    if placeholder_index < repeat.start_index() || placeholder_index >= repeat.end_index() {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` placeholder index {placeholder_index} is outside Repeat `{}` item range {}..{}",
                usage.id(),
                repeat.id(),
                repeat.start_index(),
                repeat.end_index()
            ),
        ));
    }

    Ok(placeholder_index)
}

fn mutation_repeat_param_placeholder_index(
    mutation: &core::RawMutation,
    repeat: &core::RepeatUsage,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<usize> {
    let Some(placeholder_index) = usage.placeholder_index() else {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            format!(
                "Param `{}` in Repeat `{}` is missing placeholder position metadata",
                usage.id(),
                repeat.id()
            ),
        ));
    };
    if placeholder_index < repeat.start_index() || placeholder_index >= repeat.end_index() {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            format!(
                "Param `{}` placeholder index {placeholder_index} is outside Repeat `{}` item range {}..{}",
                usage.id(),
                repeat.id(),
                repeat.start_index(),
                repeat.end_index()
            ),
        ));
    }

    Ok(placeholder_index)
}
