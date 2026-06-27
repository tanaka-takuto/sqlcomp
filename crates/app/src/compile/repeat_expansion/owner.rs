use sqlay_core as core;

use super::super::diagnostics::{
    location_error, mutation_error, mutation_param_usage_error, param_usage_error, query_error,
};
use super::super::slot_variants::{
    ExpandedFragmentParamOccurrence, ExpandedFragmentRepeatParamOccurrence, ExpandedParamBuffers,
    ExpandedParamOccurrence, ExpandedParamScope, ExpandedRepeatParamOccurrence,
};

#[derive(Clone, Copy)]
pub(super) enum RepeatOwner<'a> {
    Query(&'a core::RawQuery),
    Mutation(&'a core::RawMutation),
    FragmentQuery {
        query: &'a core::RawQuery,
        slot_usage: &'a core::SlotUsage,
        slot_occurrence_index: usize,
        fragment: &'a core::RawFragment,
    },
    FragmentMutation {
        mutation: &'a core::RawMutation,
        slot_usage: &'a core::SlotUsage,
        slot_occurrence_index: usize,
        fragment: &'a core::RawFragment,
    },
}

impl<'a> RepeatOwner<'a> {
    pub(super) fn analysis_sql(self) -> &'a str {
        match self {
            Self::Query(query) => query.analysis_sql(),
            Self::Mutation(mutation) => mutation.analysis_sql(),
            Self::FragmentQuery { fragment, .. } | Self::FragmentMutation { fragment, .. } => {
                fragment.analysis_sql()
            }
        }
    }

    pub(super) const fn source_kind(self) -> &'static str {
        match self {
            Self::Query(_) | Self::FragmentQuery { .. } => "query",
            Self::Mutation(_) | Self::FragmentMutation { .. } => "mutation",
        }
    }

    pub(super) fn source_id(self) -> &'a str {
        match self {
            Self::Query(query) | Self::FragmentQuery { query, .. } => query.metadata().id(),
            Self::Mutation(mutation) | Self::FragmentMutation { mutation, .. } => {
                mutation.metadata().id()
            }
        }
    }

    pub(super) fn slot_id(self) -> &'a str {
        match self {
            Self::FragmentQuery { slot_usage, .. } | Self::FragmentMutation { slot_usage, .. } => {
                slot_usage.id()
            }
            Self::Query(_) | Self::Mutation(_) => "<none>",
        }
    }

    pub(super) fn repeat_param_placeholder_index(
        self,
        repeat: &core::RepeatUsage,
        usage: &core::ParamUsage,
    ) -> core::DiagnosticResult<usize> {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(self.repeat_usage_error(
                repeat,
                format!(
                    "Param `{}` in Repeat `{}` for {} `{}` is missing placeholder position metadata",
                    usage.id(),
                    repeat.id(),
                    self.repeat_owner_label(),
                    self.repeat_owner_id()
                ),
            ));
        };
        if placeholder_index < repeat.start_index() || placeholder_index >= repeat.end_index() {
            return Err(self.repeat_usage_error(
                repeat,
                format!(
                    "Param `{}` placeholder index {placeholder_index} is outside Repeat `{}` item range {}..{} in {} `{}`",
                    usage.id(),
                    repeat.id(),
                    repeat.start_index(),
                    repeat.end_index(),
                    self.repeat_owner_label(),
                    self.repeat_owner_id()
                ),
            ));
        }

        Ok(placeholder_index)
    }

    pub(super) fn repeat_usage_error(
        self,
        repeat: &core::RepeatUsage,
        message: impl Into<String>,
    ) -> core::DiagnosticReport {
        let location = repeat_location(repeat, self.fallback_location());
        let mut diagnostics = location_error(location, message).into_diagnostics();

        match self {
            Self::FragmentQuery {
                query,
                slot_usage,
                fragment,
                ..
            } => diagnostics.push(
                core::Diagnostic::note(format!(
                    "Slot `{}` selected Fragment `{}` while validating query `{}`",
                    slot_usage.id(),
                    fragment.metadata().id(),
                    query.metadata().id()
                ))
                .with_location(slot_usage.source_location().clone()),
            ),
            Self::FragmentMutation {
                mutation,
                slot_usage,
                fragment,
                ..
            } => diagnostics.push(
                core::Diagnostic::note(format!(
                    "Slot `{}` selected Fragment `{}` while validating mutation `{}`",
                    slot_usage.id(),
                    fragment.metadata().id(),
                    mutation.metadata().id()
                ))
                .with_location(slot_usage.source_location().clone()),
            ),
            Self::Query(_) | Self::Mutation(_) => {}
        }

        core::DiagnosticReport::from_diagnostics(diagnostics)
    }

    pub(super) fn fragment_param_error(
        self,
        usage: &core::ParamUsage,
        message: impl Into<String>,
    ) -> core::DiagnosticReport {
        match self {
            Self::FragmentQuery { query, .. } => param_usage_error(query, usage, message),
            Self::FragmentMutation { mutation, .. } => {
                mutation_param_usage_error(mutation, usage, message)
            }
            Self::Query(query) => query_error(query, message),
            Self::Mutation(mutation) => mutation_error(mutation, message),
        }
    }

    pub(super) fn push_fragment_param_context(self, params: &mut ExpandedParamBuffers) {
        let (slot_usage, slot_occurrence_index, fragment) = match self {
            Self::FragmentQuery {
                slot_usage,
                slot_occurrence_index,
                fragment,
                ..
            }
            | Self::FragmentMutation {
                slot_usage,
                slot_occurrence_index,
                fragment,
                ..
            } => (slot_usage, slot_occurrence_index, fragment),
            Self::Query(_) | Self::Mutation(_) => return,
        };

        params.scopes.push(ExpandedParamScope::Fragment {
            slot_id: slot_usage.id().to_owned(),
            target_id: fragment.metadata().id().to_owned(),
        });
        params.occurrences.push(ExpandedParamOccurrence::Fragment(
            ExpandedFragmentParamOccurrence {
                slot_id: slot_usage.id().to_owned(),
                target_id: fragment.metadata().id().to_owned(),
                slot_occurrence_index,
                slot_location: slot_usage.source_location().clone(),
            },
        ));
    }

    pub(super) fn push_repeat_param_context(
        self,
        params: &mut ExpandedParamBuffers,
        repeat: &core::RepeatUsage,
        representative_item_index: usize,
    ) {
        match self {
            Self::Query(_) | Self::Mutation(_) => {
                params.scopes.push(ExpandedParamScope::RepeatItem {
                    repeat_id: repeat.id().to_owned(),
                });
                params.occurrences.push(ExpandedParamOccurrence::RepeatItem(
                    ExpandedRepeatParamOccurrence {
                        repeat_id: repeat.id().to_owned(),
                        representative_item_index,
                        repeat_location: repeat.source_location().clone(),
                    },
                ));
            }
            Self::FragmentQuery {
                slot_usage,
                slot_occurrence_index,
                fragment,
                ..
            }
            | Self::FragmentMutation {
                slot_usage,
                slot_occurrence_index,
                fragment,
                ..
            } => {
                params.scopes.push(ExpandedParamScope::FragmentRepeatItem {
                    slot_id: slot_usage.id().to_owned(),
                    target_id: fragment.metadata().id().to_owned(),
                    repeat_id: repeat.id().to_owned(),
                });
                params
                    .occurrences
                    .push(ExpandedParamOccurrence::FragmentRepeatItem(
                        ExpandedFragmentRepeatParamOccurrence {
                            slot_id: slot_usage.id().to_owned(),
                            target_id: fragment.metadata().id().to_owned(),
                            repeat_id: repeat.id().to_owned(),
                            representative_item_index,
                            slot_occurrence_index,
                            slot_location: slot_usage.source_location().clone(),
                            repeat_location: repeat.source_location().clone(),
                        },
                    ));
            }
        }
    }

    const fn repeat_owner_label(self) -> &'static str {
        match self {
            Self::Query(_) => "query",
            Self::Mutation(_) => "mutation",
            Self::FragmentQuery { .. } | Self::FragmentMutation { .. } => "Fragment",
        }
    }

    fn repeat_owner_id(self) -> &'a str {
        match self {
            Self::Query(query) => query.metadata().id(),
            Self::Mutation(mutation) => mutation.metadata().id(),
            Self::FragmentQuery { fragment, .. } | Self::FragmentMutation { fragment, .. } => {
                fragment.metadata().id()
            }
        }
    }

    fn fallback_location(self) -> Option<&'a core::SourceLocation> {
        match self {
            Self::Query(query) => query.source_location(),
            Self::Mutation(mutation) => mutation.source_location(),
            Self::FragmentQuery {
                query, fragment, ..
            } => fragment
                .source_location()
                .or_else(|| query.source_location()),
            Self::FragmentMutation {
                mutation, fragment, ..
            } => fragment
                .source_location()
                .or_else(|| mutation.source_location()),
        }
    }
}

fn repeat_location(
    repeat: &core::RepeatUsage,
    fallback: Option<&core::SourceLocation>,
) -> core::SourceLocation {
    if repeat.source_location().range().is_some() || repeat.source_location().path().is_some() {
        return repeat.source_location().clone();
    }

    fallback
        .cloned()
        .unwrap_or_else(core::SourceLocation::unknown)
}
