use std::collections::HashMap;

use sqlay_core as core;

use super::diagnostics::{
    location_error, param_usage_error, query_error, query_param_placeholder_index, slot_usage_error,
};
use super::param_validation::ScopedParamBinding;
use super::slot_variants::{ExpandedParamScope, SlotSpec};

pub(super) fn compile_dynamic_query_body(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledDynamicQuery> {
    let mut base_segments = Vec::with_capacity(query.slot_usages().len() + 1);
    let mut slot_occurrences = Vec::with_capacity(query.slot_usages().len());
    let mut cursor = 0;
    let mut query_param_cursor = 0;

    for usage in query.slot_usages() {
        let insertion_index = usage.insertion_index();
        if insertion_index < cursor || insertion_index > query.analysis_sql().len() {
            return Err(slot_usage_error(
                query,
                usage,
                format!(
                    "invalid Slot `{}` insertion index {insertion_index} for query analysis SQL",
                    usage.id()
                ),
            ));
        }

        base_segments.push(compiled_base_segment(
            query,
            cursor,
            insertion_index,
            &mut query_param_cursor,
            scoped_param_bindings,
        )?);
        slot_occurrences.push(core::CompiledSlotOccurrence::new(usage.id().to_owned()));
        cursor = insertion_index;
    }

    base_segments.push(compiled_base_segment(
        query,
        cursor,
        query.analysis_sql().len(),
        &mut query_param_cursor,
        scoped_param_bindings,
    )?);

    let slots = slot_specs
        .iter()
        .map(|slot| compiled_slot_definition(query, slot, fragments_by_id, scoped_param_bindings))
        .collect::<core::DiagnosticResult<Vec<_>>>()?;

    Ok(core::CompiledDynamicQuery::new(
        base_segments,
        slot_occurrences,
        slots,
    ))
}

fn compiled_base_segment(
    query: &core::RawQuery,
    segment_start: usize,
    segment_end: usize,
    query_param_cursor: &mut usize,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlSegment> {
    let Some(sql) = query.analysis_sql().get(segment_start..segment_end) else {
        return Err(query_error(
            query,
            format!(
                "invalid query SQL segment range {segment_start}..{segment_end} while compiling Slot Core IR"
            ),
        ));
    };
    let mut params = Vec::new();

    while let Some(usage) = query.param_usages().get(*query_param_cursor) {
        let placeholder_index = query_param_placeholder_index(query, usage)?;
        if placeholder_index >= segment_end {
            break;
        }
        if placeholder_index < segment_start {
            return Err(param_usage_error(
                query,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Slot Core IR segment start {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.push(compiled_param_binding(
            query,
            usage,
            &ExpandedParamScope::QueryDirect,
            scoped_param_bindings,
        )?);
        *query_param_cursor += 1;
    }

    Ok(core::CompiledSqlSegment::new(sql.to_owned(), params))
}

fn compiled_slot_definition(
    query: &core::RawQuery,
    slot: &SlotSpec,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSlotDefinition> {
    let branches = slot
        .targets
        .iter()
        .map(|target| {
            let Some(fragment) = fragments_by_id.get(target.as_str()).copied() else {
                return Err(location_error(
                    slot.source_location.clone(),
                    format!(
                        "unknown Slot target `{target}` in Slot `{}`; no fragment with that id was found",
                        slot.id
                    ),
                ));
            };
            compiled_slot_branch(query, &slot.id, fragment, scoped_param_bindings)
        })
        .collect::<core::DiagnosticResult<Vec<_>>>()?;

    Ok(core::CompiledSlotDefinition::new(slot.id.clone(), branches))
}

fn compiled_slot_branch(
    query: &core::RawQuery,
    slot_id: &str,
    fragment: &core::RawFragment,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSlotBranch> {
    let scope = ExpandedParamScope::Fragment {
        slot_id: slot_id.to_owned(),
        target_id: fragment.metadata().id().to_owned(),
    };
    let params = fragment
        .param_usages()
        .iter()
        .map(|usage| compiled_param_binding(query, usage, &scope, scoped_param_bindings))
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    let segment = core::CompiledSqlSegment::new(fragment.analysis_sql().to_owned(), params);

    Ok(core::CompiledSlotBranch::new(
        fragment.metadata().id().to_owned(),
        vec![segment],
    ))
}

fn compiled_param_binding(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    scope: &ExpandedParamScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::ParamBinding> {
    let Some(binding) = scoped_param_bindings
        .iter()
        .find(|binding| binding.scope == *scope && binding.id == usage.id())
    else {
        return Err(query_error(
            query,
            format!(
                "missing compiled Param binding for Param `{}` while compiling Slot Core IR",
                usage.id()
            ),
        ));
    };

    Ok(core::ParamBinding::new(
        usage.id().to_owned(),
        binding.ty,
        binding.nullable,
    ))
}
