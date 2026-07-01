use sqlay_core as core;

use super::ScopedParamBinding;
use crate::compile::diagnostics::{mutation_param_usage_error, param_usage_error};
use crate::compile::slot_variants::{
    ExpandedFragmentParamOccurrence, ExpandedFragmentRepeatParamOccurrence,
    ExpandedParamOccurrence, ExpandedRepeatParamOccurrence,
};

pub(super) fn param_type_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_ty: &core::CoreTypeRef,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` type in query `{}`, Repeat `{}`: first representative occurrence resolved to {:?} but conflicting representative occurrence resolved to {:?}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.repeat_id,
                existing.type_ref,
                later_ty,
            ),
        );
    }
    if let Some((first, later)) =
        fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return fragment_repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` type in query `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence resolved to {:?} but conflicting representative occurrence resolved to {:?}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                existing.type_ref,
                later_ty,
            ),
        );
    }
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
                existing.type_ref,
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
            existing.type_ref,
            later_ty
        ),
    )
}

pub(super) fn param_nullability_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_nullable: bool,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` nullability in query `{}`, Repeat `{}`: first representative occurrence is nullable {} but conflicting representative occurrence is nullable {}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.repeat_id,
                existing.nullable,
                later_nullable,
            ),
        );
    }
    if let Some((first, later)) =
        fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return fragment_repeat_param_conflict_error(
            query,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` nullability in query `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence is nullable {} but conflicting representative occurrence is nullable {}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                query.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                existing.nullable,
                later_nullable,
            ),
        );
    }
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

pub(super) fn mutation_param_type_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_ty: &core::CoreTypeRef,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return mutation_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` type in mutation `{}`, Repeat `{}`: first representative occurrence resolved to {:?} but conflicting representative occurrence resolved to {:?}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.repeat_id,
                existing.type_ref,
                later_ty,
            ),
        );
    }
    if let Some((first, later)) =
        fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return mutation_fragment_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` type in mutation `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence resolved to {:?} but conflicting representative occurrence resolved to {:?}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                existing.type_ref,
                later_ty,
            ),
        );
    }
    if let Some((first, later)) =
        repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeated_fragment_mutation_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` type in mutation `{}`, Slot `{}`, Fragment `{}`: occurrence {} resolved to {:?} but occurrence {} resolved to {:?}; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                existing.type_ref,
                later.slot_occurrence_index,
                later_ty,
            ),
        );
    }

    mutation_param_usage_error(
        mutation,
        usage,
        format!(
            "conflicting Param `{}` types: first occurrence resolved to {:?} but later occurrence resolved to {:?}",
            usage.id(),
            existing.type_ref,
            later_ty
        ),
    )
}

pub(super) fn mutation_param_nullability_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    existing: &ScopedParamBinding,
    later_nullable: bool,
    later_occurrence: &ExpandedParamOccurrence,
) -> core::DiagnosticReport {
    if let Some((first, later)) =
        repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return mutation_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Repeat item Param `{}` nullability in mutation `{}`, Repeat `{}`: first representative occurrence is nullable {} but conflicting representative occurrence is nullable {}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.repeat_id,
                existing.nullable,
                later_nullable,
            ),
        );
    }
    if let Some((first, later)) =
        fragment_repeat_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return mutation_fragment_repeat_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Repeat item Param `{}` nullability in mutation `{}`, Slot `{}`, Fragment `{}`, Repeat `{}`: first representative occurrence is nullable {} but conflicting representative occurrence is nullable {}; Repeat item fields with the same ID must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.repeat_id,
                existing.nullable,
                later_nullable,
            ),
        );
    }
    if let Some((first, later)) =
        repeated_fragment_occurrence_pair(&existing.first_occurrence, later_occurrence)
    {
        return repeated_fragment_mutation_param_conflict_error(
            mutation,
            usage,
            first,
            later,
            format!(
                "conflicting Fragment Param `{}` nullability in mutation `{}`, Slot `{}`, Fragment `{}`: occurrence {} is nullable {} but occurrence {} is nullable {}; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability",
                usage.id(),
                mutation.metadata().id(),
                first.slot_id,
                first.target_id,
                first.slot_occurrence_index,
                existing.nullable,
                later.slot_occurrence_index,
                later_nullable,
            ),
        );
    }

    mutation_param_usage_error(
        mutation,
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

fn repeat_occurrence_pair<'a>(
    first: &'a ExpandedParamOccurrence,
    later: &'a ExpandedParamOccurrence,
) -> Option<(
    &'a ExpandedRepeatParamOccurrence,
    &'a ExpandedRepeatParamOccurrence,
)> {
    let (ExpandedParamOccurrence::RepeatItem(first), ExpandedParamOccurrence::RepeatItem(later)) =
        (first, later)
    else {
        return None;
    };

    (first.repeat_id == later.repeat_id).then_some((first, later))
}

fn fragment_repeat_occurrence_pair<'a>(
    first: &'a ExpandedParamOccurrence,
    later: &'a ExpandedParamOccurrence,
) -> Option<(
    &'a ExpandedFragmentRepeatParamOccurrence,
    &'a ExpandedFragmentRepeatParamOccurrence,
)> {
    let (
        ExpandedParamOccurrence::FragmentRepeatItem(first),
        ExpandedParamOccurrence::FragmentRepeatItem(later),
    ) = (first, later)
    else {
        return None;
    };

    (first.slot_id == later.slot_id
        && first.target_id == later.target_id
        && first.repeat_id == later.repeat_id)
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

fn repeat_param_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    first: &ExpandedRepeatParamOccurrence,
    later: &ExpandedRepeatParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = param_usage_error(query, usage, message).into_diagnostics();
    diagnostics.push(
        core::Diagnostic::note(format!(
            "first Repeat `{}` occurrence is here",
            first.repeat_id
        ))
        .with_location(first.repeat_location.clone()),
    );
    diagnostics.push(
        core::Diagnostic::note(format!(
            "conflicting Repeat `{}` occurrence is here",
            later.repeat_id
        ))
        .with_location(later.repeat_location.clone()),
    );

    core::DiagnosticReport::from_diagnostics(diagnostics)
}

fn fragment_repeat_param_conflict_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    first: &ExpandedFragmentRepeatParamOccurrence,
    later: &ExpandedFragmentRepeatParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = param_usage_error(query, usage, message).into_diagnostics();
    diagnostics.push(
        core::Diagnostic::note(format!(
            "first Repeat `{}` occurrence in Slot `{}` selecting Fragment `{}` is here",
            first.repeat_id, first.slot_id, first.target_id
        ))
        .with_location(first.repeat_location.clone()),
    );
    diagnostics.push(
        core::Diagnostic::note(format!(
            "conflicting Repeat `{}` occurrence in Slot `{}` selecting Fragment `{}` is here",
            later.repeat_id, later.slot_id, later.target_id
        ))
        .with_location(later.repeat_location.clone()),
    );

    core::DiagnosticReport::from_diagnostics(diagnostics)
}

fn repeated_fragment_mutation_param_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    first: &ExpandedFragmentParamOccurrence,
    later: &ExpandedFragmentParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = mutation_param_usage_error(mutation, usage, message).into_diagnostics();
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

fn mutation_repeat_param_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    first: &ExpandedRepeatParamOccurrence,
    later: &ExpandedRepeatParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = mutation_param_usage_error(mutation, usage, message).into_diagnostics();
    diagnostics.push(
        core::Diagnostic::note(format!(
            "first Repeat `{}` occurrence is here",
            first.repeat_id
        ))
        .with_location(first.repeat_location.clone()),
    );
    diagnostics.push(
        core::Diagnostic::note(format!(
            "conflicting Repeat `{}` occurrence is here",
            later.repeat_id
        ))
        .with_location(later.repeat_location.clone()),
    );

    core::DiagnosticReport::from_diagnostics(diagnostics)
}

fn mutation_fragment_repeat_param_conflict_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    first: &ExpandedFragmentRepeatParamOccurrence,
    later: &ExpandedFragmentRepeatParamOccurrence,
    message: String,
) -> core::DiagnosticReport {
    let mut diagnostics = mutation_param_usage_error(mutation, usage, message).into_diagnostics();
    diagnostics.push(
        core::Diagnostic::note(format!(
            "first Repeat `{}` occurrence in Slot `{}` selecting Fragment `{}` is here",
            first.repeat_id, first.slot_id, first.target_id
        ))
        .with_location(first.repeat_location.clone()),
    );
    diagnostics.push(
        core::Diagnostic::note(format!(
            "conflicting Repeat `{}` occurrence in Slot `{}` selecting Fragment `{}` is here",
            later.repeat_id, later.slot_id, later.target_id
        ))
        .with_location(later.repeat_location.clone()),
    );

    core::DiagnosticReport::from_diagnostics(diagnostics)
}
