use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use sqlay_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileWriter, MetadataProvider, MutationAnalyzer,
    MutationCompiler, MutationMetadataProvider, QueryCompiler, SourceReader, TargetGenerator,
};

use super::diagnostics::{query_error, with_slot_variant_context};
use super::dynamic_ir::{compile_dynamic_mutation_body, compile_dynamic_query_body};
use super::param_validation::{
    ScopedParamBinding, validate_expanded_mutation_variant_param_bindings,
    validate_expanded_variant_param_bindings,
};
use super::slot_variants::{analyze_mutation_variants, analyze_query_variants};
use super::variant_validation::{
    validate_mutation_variant_kind, validate_variant_cardinality, validate_variant_row_shape,
};
use super::{CompilePipeline, MutationSummary, QuerySummary};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GeneratedPipelineOutput {
    pub(super) generated_files: core::GeneratedFiles,
    pub(super) diagnostics: core::DiagnosticReport,
    pub(super) source_file_count: usize,
    pub(super) output_dir: PathBuf,
    pub(super) query_summaries: Vec<QuerySummary>,
    pub(super) mutation_summaries: Vec<MutationSummary>,
    pub(super) fragment_count: usize,
}

pub(super) fn generate_files<P, S, D, M, Q, T, W>(
    plan: &core::CompilationPlan,
    pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
) -> core::DiagnosticResult<GeneratedPipelineOutput>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer + MutationAnalyzer,
    M: MetadataProvider + MutationMetadataProvider,
    Q: QueryCompiler + MutationCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    let source_read = pipeline.source_reader.read(plan)?;
    let source_file_count = source_read.source_file_count();
    let (raw_queries, raw_mutations, raw_fragments, raw_source_units, mut diagnostics) =
        source_read.into_parts();
    let source_units = source_units_or_fallback(
        &raw_queries,
        &raw_mutations,
        &raw_fragments,
        raw_source_units,
    );
    let fragment_count = raw_fragments.len();
    let fragments_by_id = raw_fragments
        .iter()
        .map(|fragment| (fragment.metadata().id(), fragment))
        .collect::<HashMap<_, _>>();
    let mut compiled_builders = Vec::with_capacity(raw_queries.len() + raw_mutations.len());
    let mut query_summaries = Vec::with_capacity(raw_queries.len());
    let mut mutation_summaries = Vec::with_capacity(raw_mutations.len());
    let mut used_fragment_ids = HashSet::new();

    for source_unit in &source_units {
        match source_unit {
            core::RawSourceUnit::Query(query) => {
                let (compiled, summary) = compile_query_builder(
                    query,
                    &fragments_by_id,
                    &mut used_fragment_ids,
                    pipeline,
                )?;
                query_summaries.push(summary);
                compiled_builders.push(core::CompiledBuilder::Query(compiled));
            }
            core::RawSourceUnit::Mutation(mutation) => {
                let (compiled, summary) = compile_mutation_builder(
                    mutation,
                    &fragments_by_id,
                    &mut used_fragment_ids,
                    pipeline,
                )?;
                mutation_summaries.push(summary);
                compiled_builders.push(core::CompiledBuilder::Mutation(compiled));
            }
            core::RawSourceUnit::Fragment(_) => {}
        }
    }

    push_unused_fragment_warnings(&raw_fragments, &used_fragment_ids, &mut diagnostics);

    let generated_files = pipeline
        .target_generator
        .generate(plan, &compiled_builders)?;

    Ok(GeneratedPipelineOutput {
        generated_files,
        diagnostics,
        source_file_count,
        output_dir: plan.output_dir().to_path_buf(),
        query_summaries,
        mutation_summaries,
        fragment_count,
    })
}

fn compile_query_builder<P, S, D, M, Q, T, W>(
    query: &core::RawQuery,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
) -> core::DiagnosticResult<(core::CompiledQuery, QuerySummary)>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer + MutationAnalyzer,
    M: MetadataProvider + MutationMetadataProvider,
    Q: QueryCompiler + MutationCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    let analyzed_variants = analyze_query_variants(
        query,
        fragments_by_id,
        used_fragment_ids,
        pipeline.dialect_analyzer,
    )?;
    let unique_slot_count = analyzed_variants.unique_slot_count;
    let variant_count = analyzed_variants.variants.len();
    let Some(base_variant) = analyzed_variants.variants.first() else {
        return Err(query_error(
            query,
            "Slot expansion produced no validation variants",
        ));
    };
    validate_variant_cardinality(&analyzed_variants.variants)?;
    let base_metadata = pipeline
        .metadata_provider
        .describe(&base_variant.query, &base_variant.analysis)
        .map_err(|report| with_slot_variant_context(report, base_variant.context.as_ref()))?;
    let mut scoped_param_bindings = Vec::<ScopedParamBinding>::new();
    validate_expanded_variant_param_bindings(
        base_variant,
        &base_metadata,
        &mut scoped_param_bindings,
    )
    .map_err(|report| with_slot_variant_context(report, base_variant.context.as_ref()))?;
    for variant in analyzed_variants.variants.iter().skip(1) {
        let metadata = pipeline
            .metadata_provider
            .describe(&variant.query, &variant.analysis)
            .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
        validate_variant_row_shape(&base_metadata, variant, &metadata)?;
        validate_expanded_variant_param_bindings(variant, &metadata, &mut scoped_param_bindings)
            .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
    }
    let compiled = pipeline.query_compiler.compile(
        &base_variant.query,
        &base_variant.analysis,
        &base_metadata,
    )?;
    let compiled = if analyzed_variants.slot_specs.is_empty() {
        compiled
    } else {
        compiled.with_dynamic_body(compile_dynamic_query_body(
            query,
            &analyzed_variants.slot_specs,
            fragments_by_id,
            &scoped_param_bindings,
        )?)
    };
    let summary = QuerySummary::from_compiled_query(&compiled, unique_slot_count, variant_count);

    Ok((compiled, summary))
}

fn compile_mutation_builder<P, S, D, M, Q, T, W>(
    mutation: &core::RawMutation,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
) -> core::DiagnosticResult<(core::CompiledMutation, MutationSummary)>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer + MutationAnalyzer,
    M: MetadataProvider + MutationMetadataProvider,
    Q: QueryCompiler + MutationCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    let analyzed_variants = analyze_mutation_variants(
        mutation,
        fragments_by_id,
        used_fragment_ids,
        pipeline.dialect_analyzer,
    )?;
    let unique_slot_count = analyzed_variants.unique_slot_count;
    let variant_count = analyzed_variants.variants.len();
    let Some(base_variant) = analyzed_variants.variants.first() else {
        return Err(super::diagnostics::mutation_error(
            mutation,
            "Slot expansion produced no validation variants",
        ));
    };
    validate_mutation_variant_kind(&analyzed_variants.variants)?;
    let base_metadata = pipeline
        .metadata_provider
        .describe_mutation(&base_variant.mutation, &base_variant.analysis)
        .map_err(|report| with_slot_variant_context(report, base_variant.context.as_ref()))?;
    let mut scoped_param_bindings = Vec::<ScopedParamBinding>::new();
    validate_expanded_mutation_variant_param_bindings(
        base_variant,
        &base_metadata,
        &mut scoped_param_bindings,
    )
    .map_err(|report| with_slot_variant_context(report, base_variant.context.as_ref()))?;
    for variant in analyzed_variants.variants.iter().skip(1) {
        let metadata = pipeline
            .metadata_provider
            .describe_mutation(&variant.mutation, &variant.analysis)
            .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
        validate_expanded_mutation_variant_param_bindings(
            variant,
            &metadata,
            &mut scoped_param_bindings,
        )
        .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
    }
    let compiled = pipeline.query_compiler.compile_mutation(
        &base_variant.mutation,
        &base_variant.analysis,
        &base_metadata,
    )?;
    let compiled = if analyzed_variants.slot_specs.is_empty() {
        compiled
    } else {
        compiled.with_dynamic_body(compile_dynamic_mutation_body(
            mutation,
            &analyzed_variants.slot_specs,
            fragments_by_id,
            &scoped_param_bindings,
        )?)
    };
    let summary =
        MutationSummary::from_compiled_mutation(&compiled, unique_slot_count, variant_count);

    Ok((compiled, summary))
}

fn source_units_or_fallback(
    queries: &[core::RawQuery],
    mutations: &[core::RawMutation],
    fragments: &[core::RawFragment],
    source_units: Vec<core::RawSourceUnit>,
) -> Vec<core::RawSourceUnit> {
    if !source_units.is_empty() {
        return source_units;
    }

    queries
        .iter()
        .cloned()
        .map(core::RawSourceUnit::Query)
        .chain(mutations.iter().cloned().map(core::RawSourceUnit::Mutation))
        .chain(fragments.iter().cloned().map(core::RawSourceUnit::Fragment))
        .collect()
}

fn push_unused_fragment_warnings(
    fragments: &[core::RawFragment],
    used_fragment_ids: &HashSet<String>,
    diagnostics: &mut core::DiagnosticReport,
) {
    for fragment in fragments {
        if used_fragment_ids.contains(fragment.metadata().id()) {
            continue;
        }

        let mut diagnostic = core::Diagnostic::warning(format!(
            "unused fragment `{}`; no Slot target references this fragment",
            fragment.metadata().id()
        ));
        if let Some(location) = fragment.source_location() {
            diagnostic = diagnostic.with_location(location.clone());
        }
        diagnostics.push(diagnostic);
    }
}
