use sqlay_core as core;

use super::diagnostics::{param_usage_error, query_error};
use super::slot_variants::{
    AnalyzedQueryVariant, ExpandedFragmentParamOccurrence, ExpandedParamOccurrence,
    ExpandedParamScope,
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

fn param_type_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_ty: core::CoreType,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeated_fragment_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` type in query `{}`, Slot `{}`, Fragment `{}`: occurrence {} resolved to {:?} but occurrence {} resolved to {:?}; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                existing.ty,
                later.slot_occurrence_index,
                later_ty,
            ),
        );
    }

    param_usage_error(
        query,
        usage,
        format!(
            "conflicting Param `{}` types: first occurrence resolved to {:?} but later occurrence resolved to {:?}",
            usage.id(),
            existing.ty,
            later_ty
        ),
    )
}

fn param_nullability_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_nullable: bool,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeated_fragment_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` nullability in query `{}`, Slot `{}`, Fragment `{}`: occurrence {} is nullable {} but occurrence {} is nullable {}; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                existing.nullable,
                later.slot_occurrence_index,
                later_nullable,
            ),
        );
    }

    param_usage_error(
        query,
        usage,
        format!(
            "conflicting Param `{}` nullability: first occurrence is nullable {} but later occurrence is nullable {}",
            usage.id(),
            existing.nullable,
            later_nullable
        ),
    )
}

fn repeated_fragment_occurrence_pair<'a>(
    first: &'a ExpandedParamOccurrence,
    later: &'a ExpandedParamOccurrence,
) -> Option<(
    &'a ExpandedFragmentParamOccurrence,
    &'a ExpandedFragmentParamOccurrence,
)> {
    let (ExpandedParamOccurrence::Fragment(first), ExpandedParamOccurrence::Fragment(later)) =
        (first, later)
    else {
        return None;
    };

    (first.slot_id == later.slot_id
        && first.target_id == later.target_id
        && first.slot_occurrence_index != later.slot_occurrence_index)
        .then_some((first, later))
}

fn repeated_fragment_param_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    first: &ExpandedFragmentParamOccurrence,
    later: &ExpandedFragmentParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = param_usage_error(query, usage, message).into_diagnostics();
    diagnostics.push(
        core::Diagnostic::note(format!(
            "first occurrence of Slot `{}` selecting Fragment `{}` is here",
            first.slot_id, first.target_id
        ))
        .with_location(first.slot_location.clone()),
    );
    diagnostics.push(
        core::Diagnostic::note(format!(
            "conflicting occurrence of Slot `{}` selecting Fragment `{}` is here",
            later.slot_id, later.target_id
        ))
        .with_location(later.slot_location.clone()),
    );

    core::DiagnosticReport::from_diagnostics(diagnostics)
}
