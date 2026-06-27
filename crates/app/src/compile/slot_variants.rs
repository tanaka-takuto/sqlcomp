use std::collections::{HashMap, HashSet};

use sqlay_core as core;

use crate::{DialectAnalyzer, MutationAnalyzer};

use super::diagnostics::{
    location_error, mutation_param_placeholder_index, mutation_param_usage_error,
    mutation_slot_spec_error as slot_spec_error, mutation_slot_usage_error, param_usage_error,
    query_error, query_param_placeholder_index, slot_usage_error, with_slot_variant_context,
};
use super::repeat_inputs::{validate_mutation_repeat_inputs, validate_query_repeat_inputs};

const SLOT_VARIANT_LIMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AnalyzedQueryVariants {
    pub(super) variants: Vec<AnalyzedQueryVariant>,
    pub(super) slot_specs: Vec<SlotSpec>,
    pub(super) unique_slot_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AnalyzedMutationVariants {
    pub(super) variants: Vec<AnalyzedMutationVariant>,
    pub(super) slot_specs: Vec<SlotSpec>,
    pub(super) unique_slot_count: usize,
}

pub(super) fn analyze_query_variants<D>(
    query: &core::RawQuery,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    dialect_analyzer: &D,
) -> core::DiagnosticResult<AnalyzedQueryVariants>
where
    D: DialectAnalyzer,
{
    if query.slot_usages().is_empty() {
        validate_query_repeat_inputs(query, &[], fragments_by_id)?;
        let direct_param_count = query.param_usages().len();
        return Ok(AnalyzedQueryVariants {
            variants: vec![AnalyzedQueryVariant {
                query: query.clone(),
                analysis: dialect_analyzer.analyze(query)?,
                context: None,
                param_scopes: vec![ExpandedParamScope::QueryDirect; direct_param_count],
                param_occurrences: vec![ExpandedParamOccurrence::QueryDirect; direct_param_count],
            }],
            slot_specs: Vec::new(),
            unique_slot_count: 0,
        });
    }

    let slot_specs = unique_slot_specs(query)?;
    reject_direct_param_slot_collisions(query, &slot_specs)?;
    validate_query_repeat_inputs(query, &slot_specs, fragments_by_id)?;
    let variant_choices =
        slot_variant_choices(query, &slot_specs, fragments_by_id, used_fragment_ids)?;
    let variants = variant_choices
        .iter()
        .map(|choices| build_slot_variant_query(query, &slot_specs, choices))
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    let mut analyzed_variants = Vec::with_capacity(variants.len());
    for variant in variants {
        let analysis = dialect_analyzer
            .analyze(&variant.query)
            .map_err(|report| with_slot_variant_context(report, Some(&variant.context)))?;
        analyzed_variants.push(AnalyzedQueryVariant {
            query: variant.query,
            analysis,
            context: Some(variant.context),
            param_scopes: variant.param_scopes,
            param_occurrences: variant.param_occurrences,
        });
    }

    Ok(AnalyzedQueryVariants {
        variants: analyzed_variants,
        unique_slot_count: slot_specs.len(),
        slot_specs,
    })
}

pub(super) fn analyze_mutation_variants<D>(
    mutation: &core::RawMutation,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    mutation_analyzer: &D,
) -> core::DiagnosticResult<AnalyzedMutationVariants>
where
    D: MutationAnalyzer,
{
    if mutation.slot_usages().is_empty() {
        validate_mutation_repeat_inputs(mutation, &[], fragments_by_id)?;
        let direct_param_count = mutation.param_usages().len();
        return Ok(AnalyzedMutationVariants {
            variants: vec![AnalyzedMutationVariant {
                mutation: mutation.clone(),
                analysis: mutation_analyzer.analyze_mutation(mutation)?,
                context: None,
                param_scopes: vec![ExpandedParamScope::QueryDirect; direct_param_count],
                param_occurrences: vec![ExpandedParamOccurrence::QueryDirect; direct_param_count],
            }],
            slot_specs: Vec::new(),
            unique_slot_count: 0,
        });
    }

    let slot_specs = unique_mutation_slot_specs(mutation)?;
    reject_mutation_direct_param_slot_collisions(mutation, &slot_specs)?;
    validate_mutation_repeat_inputs(mutation, &slot_specs, fragments_by_id)?;
    let variant_choices =
        mutation_slot_variant_choices(mutation, &slot_specs, fragments_by_id, used_fragment_ids)?;
    let variants = variant_choices
        .iter()
        .map(|choices| build_slot_variant_mutation(mutation, &slot_specs, choices))
        .collect::<core::DiagnosticResult<Vec<_>>>()?;
    let mut analyzed_variants = Vec::with_capacity(variants.len());
    for variant in variants {
        let analysis = mutation_analyzer
            .analyze_mutation(&variant.mutation)
            .map_err(|report| with_slot_variant_context(report, Some(&variant.context)))?;
        analyzed_variants.push(AnalyzedMutationVariant {
            mutation: variant.mutation,
            analysis,
            context: Some(variant.context),
            param_scopes: variant.param_scopes,
            param_occurrences: variant.param_occurrences,
        });
    }

    Ok(AnalyzedMutationVariants {
        variants: analyzed_variants,
        unique_slot_count: slot_specs.len(),
        slot_specs,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AnalyzedQueryVariant {
    pub(super) query: core::RawQuery,
    pub(super) analysis: core::AnalyzedQuery,
    pub(super) context: Option<SlotExpansionContext>,
    pub(super) param_scopes: Vec<ExpandedParamScope>,
    pub(super) param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AnalyzedMutationVariant {
    pub(super) mutation: core::RawMutation,
    pub(super) analysis: core::AnalyzedMutation,
    pub(super) context: Option<SlotExpansionContext>,
    pub(super) param_scopes: Vec<ExpandedParamScope>,
    pub(super) param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SlotSpec {
    pub(super) id: String,
    pub(super) targets: Vec<String>,
    pub(super) source_location: core::SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotExpansionVariant {
    query: core::RawQuery,
    context: SlotExpansionContext,
    param_scopes: Vec<ExpandedParamScope>,
    param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotExpansionMutationVariant {
    mutation: core::RawMutation,
    context: SlotExpansionContext,
    param_scopes: Vec<ExpandedParamScope>,
    param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ExpandedParamScope {
    QueryDirect,
    Fragment { slot_id: String, target_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ExpandedParamOccurrence {
    QueryDirect,
    Fragment(ExpandedFragmentParamOccurrence),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExpandedFragmentParamOccurrence {
    pub(super) slot_id: String,
    pub(super) target_id: String,
    pub(super) slot_occurrence_index: usize,
    pub(super) slot_location: core::SourceLocation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ExpandedParamBuffers {
    usages: Vec<core::ParamUsage>,
    scopes: Vec<ExpandedParamScope>,
    occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SlotExpansionContext {
    pub(super) source_kind: SlotExpansionSourceKind,
    pub(super) source_id: String,
    pub(super) selections: Vec<SlotSelectionContext>,
}

impl SlotExpansionContext {
    pub(super) fn diagnostics(&self) -> Vec<core::Diagnostic> {
        let source_kind = self.source_kind.label();
        let selection_summary = self
            .selections
            .iter()
            .map(|selection| {
                let target = selection.target_id.as_deref().unwrap_or("<unselected>");
                format!("{}={target}", selection.slot_id)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut diagnostics = vec![core::Diagnostic::note(format!(
            "while validating Slot expansion variant for {source_kind} `{}` with selections: {selection_summary}",
            self.source_id
        ))];

        for selection in &self.selections {
            let target = selection.target_id.as_deref().unwrap_or("<unselected>");
            diagnostics.push(
                core::Diagnostic::note(format!(
                    "Slot `{}` selected `{target}` in this variant",
                    selection.slot_id
                ))
                .with_location(selection.slot_location.clone()),
            );
            if let Some(fragment_location) = &selection.fragment_location {
                diagnostics.push(
                    core::Diagnostic::note(format!("selected fragment `{target}` is defined here"))
                        .with_location(fragment_location.clone()),
                );
            }
        }

        diagnostics
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SlotExpansionSourceKind {
    Query,
    Mutation,
}

impl SlotExpansionSourceKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Mutation => "mutation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SlotSelectionContext {
    pub(super) slot_id: String,
    pub(super) target_id: Option<String>,
    pub(super) slot_location: core::SourceLocation,
    pub(super) fragment_location: Option<core::SourceLocation>,
}

fn unique_slot_specs(query: &core::RawQuery) -> core::DiagnosticResult<Vec<SlotSpec>> {
    let mut slot_specs = Vec::<SlotSpec>::new();

    for usage in query.slot_usages() {
        let mut seen_targets = HashSet::new();
        for target in usage.targets() {
            if !seen_targets.insert(target.as_str()) {
                return Err(slot_usage_error(
                    query,
                    usage,
                    format!(
                        "duplicate Slot target `{target}` in Slot `{}`; each target must appear at most once in `targets`",
                        usage.id()
                    ),
                ));
            }
        }

        if let Some(existing) = slot_specs.iter().find(|slot| slot.id == usage.id()) {
            if existing.targets != usage.targets() {
                return Err(slot_usage_error(
                    query,
                    usage,
                    format!(
                        "conflicting Slot `{}` targets in query `{}`: first occurrence uses {} but conflicting occurrence uses {}; repeated Slot IDs must use the same `targets` values in the same order",
                        usage.id(),
                        query.metadata().id(),
                        format_slot_targets(&existing.targets),
                        format_slot_targets(usage.targets()),
                    ),
                ));
            }
            continue;
        }

        slot_specs.push(SlotSpec {
            id: usage.id().to_owned(),
            targets: usage.targets().to_vec(),
            source_location: usage.source_location().clone(),
        });
    }

    Ok(slot_specs)
}

fn unique_mutation_slot_specs(
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<Vec<SlotSpec>> {
    let mut slot_specs = Vec::<SlotSpec>::new();

    for usage in mutation.slot_usages() {
        let mut seen_targets = HashSet::new();
        for target in usage.targets() {
            if !seen_targets.insert(target.as_str()) {
                return Err(mutation_slot_usage_error(
                    mutation,
                    usage,
                    format!(
                        "duplicate Slot target `{target}` in Slot `{}`; each target must appear at most once in `targets`",
                        usage.id()
                    ),
                ));
            }
        }

        if let Some(existing) = slot_specs.iter().find(|slot| slot.id == usage.id()) {
            if existing.targets != usage.targets() {
                return Err(mutation_slot_usage_error(
                    mutation,
                    usage,
                    format!(
                        "conflicting Slot `{}` targets in mutation `{}`: first occurrence uses {} but conflicting occurrence uses {}; repeated Slot IDs must use the same `targets` values in the same order",
                        usage.id(),
                        mutation.metadata().id(),
                        format_slot_targets(&existing.targets),
                        format_slot_targets(usage.targets()),
                    ),
                ));
            }
            continue;
        }

        slot_specs.push(SlotSpec {
            id: usage.id().to_owned(),
            targets: usage.targets().to_vec(),
            source_location: usage.source_location().clone(),
        });
    }

    Ok(slot_specs)
}

fn reject_direct_param_slot_collisions(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
) -> core::DiagnosticResult<()> {
    let direct_param_ids = query
        .param_usages()
        .iter()
        .map(core::ParamUsage::id)
        .collect::<HashSet<_>>();

    for slot in slot_specs {
        if direct_param_ids.contains(slot.id.as_str()) {
            return Err(location_error(
                slot.source_location.clone(),
                format!(
                    "Slot `{}` in query `{}` conflicts with query direct Param `{}`; query direct Param IDs and Slot IDs share the generated input namespace",
                    slot.id,
                    query.metadata().id(),
                    slot.id
                ),
            ));
        }
    }

    Ok(())
}

fn reject_mutation_direct_param_slot_collisions(
    mutation: &core::RawMutation,
    slot_specs: &[SlotSpec],
) -> core::DiagnosticResult<()> {
    let direct_param_ids = mutation
        .param_usages()
        .iter()
        .map(core::ParamUsage::id)
        .collect::<HashSet<_>>();

    for slot in slot_specs {
        if direct_param_ids.contains(slot.id.as_str()) {
            return Err(slot_spec_error(
                mutation,
                &slot.source_location,
                format!(
                    "Slot `{}` in mutation `{}` conflicts with mutation direct Param `{}`; mutation direct Param IDs and Slot IDs share the generated input namespace",
                    slot.id,
                    mutation.metadata().id(),
                    slot.id
                ),
            ));
        }
    }

    Ok(())
}

fn format_slot_targets(targets: &[String]) -> String {
    format!("[{}]", targets.join(", "))
}

fn slot_variant_choices<'a>(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &'a core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
) -> core::DiagnosticResult<Vec<Vec<Option<&'a core::RawFragment>>>> {
    let mut variants = vec![Vec::new()];

    for slot in slot_specs {
        let mut choices = Vec::with_capacity(slot.targets.len() + 1);
        choices.push(None);
        for target in &slot.targets {
            let Some(fragment) = fragments_by_id.get(target.as_str()).copied() else {
                return Err(location_error(
                    slot.source_location.clone(),
                    format!(
                        "unknown Slot target `{target}` in Slot `{}`; no fragment with that id was found",
                        slot.id
                    ),
                ));
            };
            used_fragment_ids.insert(target.clone());
            choices.push(Some(fragment));
        }

        let variant_count = variants.len().saturating_mul(choices.len());
        if variant_count > SLOT_VARIANT_LIMIT {
            return Err(query_error(
                query,
                format!(
                    "Slot expansion for query `{}` would produce {variant_count} SQL variants, exceeding the {SLOT_VARIANT_LIMIT} variant limit",
                    query.metadata().id()
                ),
            ));
        }

        let mut next_variants = Vec::with_capacity(variant_count);
        for variant in &variants {
            for choice in &choices {
                let mut next_variant = variant.clone();
                next_variant.push(*choice);
                next_variants.push(next_variant);
            }
        }
        variants = next_variants;
    }

    Ok(variants)
}

fn mutation_slot_variant_choices<'a>(
    mutation: &core::RawMutation,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &'a core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
) -> core::DiagnosticResult<Vec<Vec<Option<&'a core::RawFragment>>>> {
    let mut variants = vec![Vec::new()];

    for slot in slot_specs {
        let mut choices = Vec::with_capacity(slot.targets.len() + 1);
        choices.push(None);
        for target in &slot.targets {
            let Some(fragment) = fragments_by_id.get(target.as_str()).copied() else {
                return Err(slot_spec_error(
                    mutation,
                    &slot.source_location,
                    format!(
                        "unknown Slot target `{target}` in Slot `{}`; no fragment with that id was found",
                        slot.id
                    ),
                ));
            };
            used_fragment_ids.insert(target.clone());
            choices.push(Some(fragment));
        }

        let variant_count = variants.len().saturating_mul(choices.len());
        if variant_count > SLOT_VARIANT_LIMIT {
            return Err(super::diagnostics::mutation_error(
                mutation,
                format!(
                    "Slot expansion for mutation `{}` would produce {variant_count} SQL variants, exceeding the {SLOT_VARIANT_LIMIT} variant limit",
                    mutation.metadata().id()
                ),
            ));
        }

        let mut next_variants = Vec::with_capacity(variant_count);
        for variant in &variants {
            for choice in &choices {
                let mut next_variant = variant.clone();
                next_variant.push(*choice);
                next_variants.push(next_variant);
            }
        }
        variants = next_variants;
    }

    Ok(variants)
}

fn build_slot_variant_query(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    choices: &[Option<&core::RawFragment>],
) -> core::DiagnosticResult<SlotExpansionVariant> {
    let choices_by_slot = slot_specs
        .iter()
        .zip(choices.iter().copied())
        .map(|(slot, choice)| (slot.id.as_str(), choice))
        .collect::<HashMap<_, _>>();
    let mut analysis_sql = String::with_capacity(query.analysis_sql().len());
    let mut cursor = 0;
    let mut query_param_cursor = 0;
    let mut params = ExpandedParamBuffers::default();
    let mut slot_occurrence_counts = HashMap::<&str, usize>::new();

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

        let segment_output_start = analysis_sql.len();
        analysis_sql.push_str(&query.analysis_sql()[cursor..insertion_index]);
        push_query_params_before_index(
            query,
            cursor,
            segment_output_start,
            insertion_index,
            &mut query_param_cursor,
            &mut params,
        )?;
        if let Some(Some(fragment)) = choices_by_slot.get(usage.id()) {
            let slot_occurrence_index = slot_occurrence_counts.entry(usage.id()).or_insert(0);
            *slot_occurrence_index += 1;
            let fragment_output_start = analysis_sql.len();
            analysis_sql.push_str(fragment.analysis_sql());
            push_fragment_params(
                fragment,
                fragment_output_start,
                &mut params,
                query,
                usage,
                *slot_occurrence_index,
            )?;
        }
        cursor = insertion_index;
    }
    let segment_output_start = analysis_sql.len();
    analysis_sql.push_str(&query.analysis_sql()[cursor..]);
    push_query_params_before_index(
        query,
        cursor,
        segment_output_start,
        query.analysis_sql().len(),
        &mut query_param_cursor,
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

    Ok(SlotExpansionVariant {
        query: expanded_query,
        context: slot_expansion_context(
            SlotExpansionSourceKind::Query,
            query.metadata().id(),
            slot_specs,
            choices,
        ),
        param_scopes: params.scopes,
        param_occurrences: params.occurrences,
    })
}

fn build_slot_variant_mutation(
    mutation: &core::RawMutation,
    slot_specs: &[SlotSpec],
    choices: &[Option<&core::RawFragment>],
) -> core::DiagnosticResult<SlotExpansionMutationVariant> {
    let choices_by_slot = slot_specs
        .iter()
        .zip(choices.iter().copied())
        .map(|(slot, choice)| (slot.id.as_str(), choice))
        .collect::<HashMap<_, _>>();
    let mut analysis_sql = String::with_capacity(mutation.analysis_sql().len());
    let mut cursor = 0;
    let mut mutation_param_cursor = 0;
    let mut params = ExpandedParamBuffers::default();
    let mut slot_occurrence_counts = HashMap::<&str, usize>::new();

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

        let segment_output_start = analysis_sql.len();
        analysis_sql.push_str(&mutation.analysis_sql()[cursor..insertion_index]);
        push_mutation_params_before_index(
            mutation,
            cursor,
            segment_output_start,
            insertion_index,
            &mut mutation_param_cursor,
            &mut params,
        )?;
        if let Some(Some(fragment)) = choices_by_slot.get(usage.id()) {
            let slot_occurrence_index = slot_occurrence_counts.entry(usage.id()).or_insert(0);
            *slot_occurrence_index += 1;
            let fragment_output_start = analysis_sql.len();
            analysis_sql.push_str(fragment.analysis_sql());
            push_mutation_fragment_params(
                fragment,
                fragment_output_start,
                &mut params,
                mutation,
                usage,
                *slot_occurrence_index,
            )?;
        }
        cursor = insertion_index;
    }
    let segment_output_start = analysis_sql.len();
    analysis_sql.push_str(&mutation.analysis_sql()[cursor..]);
    push_mutation_params_before_index(
        mutation,
        cursor,
        segment_output_start,
        mutation.analysis_sql().len(),
        &mut mutation_param_cursor,
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

    Ok(SlotExpansionMutationVariant {
        mutation: expanded_mutation,
        context: slot_expansion_context(
            SlotExpansionSourceKind::Mutation,
            mutation.metadata().id(),
            slot_specs,
            choices,
        ),
        param_scopes: params.scopes,
        param_occurrences: params.occurrences,
    })
}

fn slot_expansion_context(
    source_kind: SlotExpansionSourceKind,
    source_id: &str,
    slot_specs: &[SlotSpec],
    choices: &[Option<&core::RawFragment>],
) -> SlotExpansionContext {
    let selections = slot_specs
        .iter()
        .zip(choices.iter().copied())
        .map(|(slot, choice)| SlotSelectionContext {
            slot_id: slot.id.clone(),
            target_id: choice.map(|fragment| fragment.metadata().id().to_owned()),
            slot_location: slot.source_location.clone(),
            fragment_location: choice.and_then(|fragment| fragment.source_location().cloned()),
        })
        .collect();

    SlotExpansionContext {
        source_kind,
        source_id: source_id.to_owned(),
        selections,
    }
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

fn push_fragment_params(
    fragment: &core::RawFragment,
    fragment_output_start: usize,
    params: &mut ExpandedParamBuffers,
    query: &core::RawQuery,
    slot_usage: &core::SlotUsage,
    slot_occurrence_index: usize,
) -> core::DiagnosticResult<()> {
    for usage in fragment.param_usages() {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(query_error(
                query,
                format!(
                    "Param `{}` in fragment `{}` is missing placeholder position metadata",
                    usage.id(),
                    fragment.metadata().id()
                ),
            ));
        };

        params.usages.push(
            usage
                .clone()
                .with_placeholder_index(fragment_output_start + placeholder_index),
        );
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

    Ok(())
}

fn push_mutation_fragment_params(
    fragment: &core::RawFragment,
    fragment_output_start: usize,
    params: &mut ExpandedParamBuffers,
    mutation: &core::RawMutation,
    slot_usage: &core::SlotUsage,
    slot_occurrence_index: usize,
) -> core::DiagnosticResult<()> {
    for usage in fragment.param_usages() {
        let Some(placeholder_index) = usage.placeholder_index() else {
            return Err(super::diagnostics::mutation_error(
                mutation,
                format!(
                    "Param `{}` in fragment `{}` is missing placeholder position metadata",
                    usage.id(),
                    fragment.metadata().id()
                ),
            ));
        };

        params.usages.push(
            usage
                .clone()
                .with_placeholder_index(fragment_output_start + placeholder_index),
        );
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

    Ok(())
}
