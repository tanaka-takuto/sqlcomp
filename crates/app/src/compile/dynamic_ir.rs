use std::collections::HashMap;

use sqlay_core as core;

use super::diagnostics::{
    location_error, mutation_param_placeholder_index, mutation_param_usage_error,
    mutation_slot_usage_error, param_usage_error, query_error, query_param_placeholder_index,
    slot_usage_error,
};
use super::param_validation::ScopedParamBinding;
use super::slot_variants::{ExpandedParamScope, SlotSpec};

mod repeat_bodies;
mod repeat_definitions;

use repeat_bodies::{
    compiled_mutation_body, compiled_mutation_fragment_body, compiled_query_body,
    compiled_query_fragment_body,
};
use repeat_definitions::{compiled_mutation_repeat_definitions, compiled_query_repeat_definitions};

pub(super) fn compile_dynamic_query_body(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledDynamicQuery> {
    let mut base_bodies = Vec::with_capacity(query.slot_usages().len() + 1);
    let mut slot_occurrences = Vec::with_capacity(query.slot_usages().len());
    let mut cursor = 0;
    let mut query_param_cursor = 0;
    let mut query_repeat_cursor = 0;

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

        base_bodies.push(compiled_query_body(
            query,
            cursor,
            insertion_index,
            &mut query_param_cursor,
            &mut query_repeat_cursor,
            scoped_param_bindings,
        )?);
        slot_occurrences.push(core::CompiledSlotOccurrence::new(usage.id().to_owned()));
        cursor = insertion_index;
    }

    base_bodies.push(compiled_query_body(
        query,
        cursor,
        query.analysis_sql().len(),
        &mut query_param_cursor,
        &mut query_repeat_cursor,
        scoped_param_bindings,
    )?);

    let slots = slot_specs
        .iter()
        .map(|slot| compiled_slot_definition(query, slot, fragments_by_id, scoped_param_bindings))
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    let repeats = compiled_query_repeat_definitions(
        query,
        query.repeat_usages(),
        &RepeatBindingScope::Builder,
        scoped_param_bindings,
    )?;

    Ok(core::CompiledDynamicQuery::new_with_bodies(
        base_bodies,
        slot_occurrences,
        slots,
        repeats,
    ))
}

pub(super) fn compile_dynamic_mutation_body(
    mutation: &core::RawMutation,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledDynamicQuery> {
    let mut base_bodies = Vec::with_capacity(mutation.slot_usages().len() + 1);
    let mut slot_occurrences = Vec::with_capacity(mutation.slot_usages().len());
    let mut cursor = 0;
    let mut mutation_param_cursor = 0;
    let mut mutation_repeat_cursor = 0;

    for usage in mutation.slot_usages() {
        let insertion_index = usage.insertion_index();
        if insertion_index < cursor || insertion_index > mutation.analysis_sql().len() {
            return Err(mutation_slot_usage_error(
                mutation,
                usage,
                format!(
                    "invalid Slot `{}` insertion index {insertion_index} for mutation analysis SQL",
                    usage.id()
                ),
            ));
        }

        base_bodies.push(compiled_mutation_body(
            mutation,
            cursor,
            insertion_index,
            &mut mutation_param_cursor,
            &mut mutation_repeat_cursor,
            scoped_param_bindings,
        )?);
        slot_occurrences.push(core::CompiledSlotOccurrence::new(usage.id().to_owned()));
        cursor = insertion_index;
    }

    base_bodies.push(compiled_mutation_body(
        mutation,
        cursor,
        mutation.analysis_sql().len(),
        &mut mutation_param_cursor,
        &mut mutation_repeat_cursor,
        scoped_param_bindings,
    )?);

    let slots = slot_specs
        .iter()
        .map(|slot| {
            compiled_mutation_slot_definition(
                mutation,
                slot,
                fragments_by_id,
                scoped_param_bindings,
            )
        })
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    let repeats = compiled_mutation_repeat_definitions(
        mutation,
        mutation.repeat_usages(),
        &RepeatBindingScope::Builder,
        scoped_param_bindings,
    )?;

    Ok(core::CompiledDynamicQuery::new_with_bodies(
        base_bodies,
        slot_occurrences,
        slots,
        repeats,
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RepeatBindingScope {
    Builder,
    Fragment { slot_id: String, target_id: String },
}

impl RepeatBindingScope {
    fn expanded_scope(&self, repeat_id: &str) -> ExpandedParamScope {
        match self {
            Self::Builder => ExpandedParamScope::RepeatItem {
                repeat_id: repeat_id.to_owned(),
            },
            Self::Fragment { slot_id, target_id } => ExpandedParamScope::FragmentRepeatItem {
                slot_id: slot_id.clone(),
                target_id: target_id.clone(),
                repeat_id: repeat_id.to_owned(),
            },
        }
    }
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

fn compiled_mutation_base_segment(
    mutation: &core::RawMutation,
    segment_start: usize,
    segment_end: usize,
    mutation_param_cursor: &mut usize,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSqlSegment> {
    let Some(sql) = mutation.analysis_sql().get(segment_start..segment_end) else {
        return Err(super::diagnostics::mutation_error(
            mutation,
            format!(
                "invalid mutation SQL segment range {segment_start}..{segment_end} while compiling Slot Core IR"
            ),
        ));
    };
    let mut params = Vec::new();

    while let Some(usage) = mutation.param_usages().get(*mutation_param_cursor) {
        let placeholder_index = mutation_param_placeholder_index(mutation, usage)?;
        if placeholder_index >= segment_end {
            break;
        }
        if placeholder_index < segment_start {
            return Err(mutation_param_usage_error(
                mutation,
                usage,
                format!(
                    "Param `{}` placeholder index {placeholder_index} appears before the current Slot Core IR segment start {segment_start}",
                    usage.id()
                ),
            ));
        }

        params.push(compiled_mutation_param_binding(
            mutation,
            usage,
            &ExpandedParamScope::QueryDirect,
            scoped_param_bindings,
        )?);
        *mutation_param_cursor += 1;
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

fn compiled_mutation_slot_definition(
    mutation: &core::RawMutation,
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
            compiled_mutation_slot_branch(mutation, &slot.id, fragment, scoped_param_bindings)
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
    let repeat_scope = RepeatBindingScope::Fragment {
        slot_id: slot_id.to_owned(),
        target_id: fragment.metadata().id().to_owned(),
    };
    let body = compiled_query_fragment_body(
        query,
        fragment,
        &scope,
        &repeat_scope,
        scoped_param_bindings,
    )?;
    let repeats = compiled_query_repeat_definitions(
        query,
        fragment.repeat_usages(),
        &repeat_scope,
        scoped_param_bindings,
    )?;

    Ok(core::CompiledSlotBranch::new_with_body(
        fragment.metadata().id().to_owned(),
        body,
        repeats,
    ))
}

fn compiled_mutation_slot_branch(
    mutation: &core::RawMutation,
    slot_id: &str,
    fragment: &core::RawFragment,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::CompiledSlotBranch> {
    let scope = ExpandedParamScope::Fragment {
        slot_id: slot_id.to_owned(),
        target_id: fragment.metadata().id().to_owned(),
    };
    let repeat_scope = RepeatBindingScope::Fragment {
        slot_id: slot_id.to_owned(),
        target_id: fragment.metadata().id().to_owned(),
    };
    let body = compiled_mutation_fragment_body(
        mutation,
        fragment,
        &scope,
        &repeat_scope,
        scoped_param_bindings,
    )?;
    let repeats = compiled_mutation_repeat_definitions(
        mutation,
        fragment.repeat_usages(),
        &repeat_scope,
        scoped_param_bindings,
    )?;

    Ok(core::CompiledSlotBranch::new_with_body(
        fragment.metadata().id().to_owned(),
        body,
        repeats,
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

fn compiled_mutation_param_binding(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    scope: &ExpandedParamScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<core::ParamBinding> {
    let Some(binding) = scoped_param_bindings
        .iter()
        .find(|binding| binding.scope == *scope && binding.id == usage.id())
    else {
        return Err(super::diagnostics::mutation_error(
            mutation,
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
