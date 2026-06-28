use sqlay_core as core;

use super::super::param_validation::ScopedParamBinding;
use super::{RepeatBindingScope, compiled_mutation_param_binding, compiled_param_binding};

pub(super) fn compiled_query_repeat_definitions(
    query: &core::RawQuery,
    repeat_usages: &[core::RepeatUsage],
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::CompiledRepeatDefinition>> {
    let mut definitions = Vec::new();

    for repeat in repeat_usages {
        if definitions
            .iter()
            .any(|definition: &core::CompiledRepeatDefinition| definition.id() == repeat.id())
        {
            continue;
        }

        definitions.push(core::CompiledRepeatDefinition::new(
            repeat.id().to_owned(),
            compiled_query_repeat_fields(query, repeat, scope, scoped_param_bindings)?,
        ));
    }

    Ok(definitions)
}

pub(super) fn compiled_mutation_repeat_definitions(
    mutation: &core::RawMutation,
    repeat_usages: &[core::RepeatUsage],
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::CompiledRepeatDefinition>> {
    let mut definitions = Vec::new();

    for repeat in repeat_usages {
        if definitions
            .iter()
            .any(|definition: &core::CompiledRepeatDefinition| definition.id() == repeat.id())
        {
            continue;
        }

        definitions.push(core::CompiledRepeatDefinition::new(
            repeat.id().to_owned(),
            compiled_mutation_repeat_fields(mutation, repeat, scope, scoped_param_bindings)?,
        ));
    }

    Ok(definitions)
}

fn compiled_query_repeat_fields(
    query: &core::RawQuery,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::ParamBinding>> {
    let mut fields = Vec::new();
    let param_scope = scope.expanded_scope(repeat.id());

    for usage in repeat.item_param_usages() {
        if fields
            .iter()
            .any(|field: &core::ParamBinding| field.input_name() == usage.id())
        {
            continue;
        }

        fields.push(compiled_param_binding(
            query,
            usage,
            &param_scope,
            scoped_param_bindings,
        )?);
    }

    Ok(fields)
}

fn compiled_mutation_repeat_fields(
    mutation: &core::RawMutation,
    repeat: &core::RepeatUsage,
    scope: &RepeatBindingScope,
    scoped_param_bindings: &[ScopedParamBinding],
) -> core::DiagnosticResult<Vec<core::ParamBinding>> {
    let mut fields = Vec::new();
    let param_scope = scope.expanded_scope(repeat.id());

    for usage in repeat.item_param_usages() {
        if fields
            .iter()
            .any(|field: &core::ParamBinding| field.input_name() == usage.id())
        {
            continue;
        }

        fields.push(compiled_mutation_param_binding(
            mutation,
            usage,
            &param_scope,
            scoped_param_bindings,
        )?);
    }

    Ok(fields)
}
