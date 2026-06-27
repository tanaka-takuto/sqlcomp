use sqlay_core as core;

use super::diagnostics::{
    mutation_error, mutation_param_usage_error, param_usage_error, query_error,
};
use super::slot_variants::{
    AnalyzedMutationVariant, AnalyzedQueryVariant, ExpandedParamOccurrence, ExpandedParamScope,
};

mod conflicts;

use conflicts::{
    mutation_param_nullability_conflict_error, mutation_param_type_conflict_error,
    param_nullability_conflict_error, param_type_conflict_error,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ScopedParamBinding {
    pub(super) scope: ExpandedParamScope,
    pub(super) id: String,
    pub(super) ty: core::CoreType,
    pub(super) nullable: bool,
    pub(super) first_occurrence: ExpandedParamOccurrence,
}

pub(super) fn validate_expanded_variant_param_bindings(
    variant: &AnalyzedQueryVariant,
    metadata: &core::DbQueryMetadata,
    scoped_bindings: &mut Vec<ScopedParamBinding>,
) -> core::DiagnosticResult<()> {
    let query = &variant.query;
    if query.param_usages().len() != metadata.param_usages().len() {
        return Err(query_error(
            query,
            format!(
                "resolved Param usage count {} does not match source Param usage count {}",
                metadata.param_usages().len(),
                query.param_usages().len()
            ),
        ));
    }
    if query.param_usages().len() != variant.param_scopes.len() {
        return Err(query_error(
            query,
            format!(
                "expanded Param scope count {} does not match source Param usage count {}",
                variant.param_scopes.len(),
                query.param_usages().len()
            ),
        ));
    }
    if query.param_usages().len() != variant.param_occurrences.len() {
        return Err(query_error(
            query,
            format!(
                "expanded Param occurrence count {} does not match source Param usage count {}",
                variant.param_occurrences.len(),
                query.param_usages().len()
            ),
        ));
    }

    for (((source_usage, resolved_usage), scope), occurrence) in query
        .param_usages()
        .iter()
        .zip(metadata.param_usages())
        .zip(&variant.param_scopes)
        .zip(&variant.param_occurrences)
    {
        if source_usage.id() != resolved_usage.id() {
            return Err(param_usage_error(
                query,
                source_usage,
                format!(
                    "resolved Param metadata id `{}` does not match source Param id `{}`",
                    resolved_usage.id(),
                    source_usage.id()
                ),
            ));
        }

        let nullable = source_usage.nullable_override();
        if let Some(existing) = scoped_bindings
            .iter()
            .find(|binding| binding.scope == *scope && binding.id == source_usage.id())
        {
            if existing.ty != resolved_usage.ty() {
                return Err(param_type_conflict_error(
                    query,
                    source_usage,
                    existing,
                    resolved_usage.ty(),
                    occurrence,
                ));
            }
            if existing.nullable != nullable {
                return Err(param_nullability_conflict_error(
                    query,
                    source_usage,
                    existing,
                    nullable,
                    occurrence,
                ));
            }
        } else {
            scoped_bindings.push(ScopedParamBinding {
                scope: scope.clone(),
                id: source_usage.id().to_owned(),
                ty: resolved_usage.ty(),
                nullable,
                first_occurrence: occurrence.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn validate_expanded_mutation_variant_param_bindings(
    variant: &AnalyzedMutationVariant,
    metadata: &core::DbMutationMetadata,
    scoped_bindings: &mut Vec<ScopedParamBinding>,
) -> core::DiagnosticResult<()> {
    let mutation = &variant.mutation;
    if mutation.param_usages().len() != metadata.param_usages().len() {
        return Err(mutation_error(
            mutation,
            format!(
                "resolved Param usage count {} does not match source Param usage count {}",
                metadata.param_usages().len(),
                mutation.param_usages().len()
            ),
        ));
    }
    if mutation.param_usages().len() != variant.param_scopes.len() {
        return Err(mutation_error(
            mutation,
            format!(
                "expanded Param scope count {} does not match source Param usage count {}",
                variant.param_scopes.len(),
                mutation.param_usages().len()
            ),
        ));
    }
    if mutation.param_usages().len() != variant.param_occurrences.len() {
        return Err(mutation_error(
            mutation,
            format!(
                "expanded Param occurrence count {} does not match source Param usage count {}",
                variant.param_occurrences.len(),
                mutation.param_usages().len()
            ),
        ));
    }

    for (((source_usage, resolved_usage), scope), occurrence) in mutation
        .param_usages()
        .iter()
        .zip(metadata.param_usages())
        .zip(&variant.param_scopes)
        .zip(&variant.param_occurrences)
    {
        if source_usage.id() != resolved_usage.id() {
            return Err(mutation_param_usage_error(
                mutation,
                source_usage,
                format!(
                    "resolved Param metadata id `{}` does not match source Param id `{}`",
                    resolved_usage.id(),
                    source_usage.id()
                ),
            ));
        }

        let nullable = source_usage.nullable_override();
        if let Some(existing) = scoped_bindings
            .iter()
            .find(|binding| binding.scope == *scope && binding.id == source_usage.id())
        {
            if existing.ty != resolved_usage.ty() {
                return Err(mutation_param_type_conflict_error(
                    mutation,
                    source_usage,
                    existing,
                    resolved_usage.ty(),
                    occurrence,
                ));
            }
            if existing.nullable != nullable {
                return Err(mutation_param_nullability_conflict_error(
                    mutation,
                    source_usage,
                    existing,
                    nullable,
                    occurrence,
                ));
            }
        } else {
            scoped_bindings.push(ScopedParamBinding {
                scope: scope.clone(),
                id: source_usage.id().to_owned(),
                ty: resolved_usage.ty(),
                nullable,
                first_occurrence: occurrence.clone(),
            });
        }
    }

    Ok(())
}
