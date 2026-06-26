use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use sqlay_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileWriter, MetadataProvider, QueryCompiler,
    SourceReader, TargetGenerator,
};

use super::diagnostics::{query_error, with_slot_variant_context};
use super::dynamic_ir::compile_dynamic_query_body;
use super::param_validation::{ScopedParamBinding, validate_expanded_variant_param_bindings};
use super::slot_variants::analyze_query_variants;
use super::variant_validation::{validate_variant_cardinality, validate_variant_row_shape};
use super::{CompilePipeline, QuerySummary};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GeneratedPipelineOutput {
    pub(super) generated_files: core::GeneratedFiles,
    pub(super) diagnostics: core::DiagnosticReport,
    pub(super) source_file_count: usize,
    pub(super) output_dir: PathBuf,
    pub(super) query_summaries: Vec<QuerySummary>,
    pub(super) fragment_count: usize,
}

pub(super) fn generate_files<P, S, D, M, Q, T, W>(
    plan: &core::CompilationPlan,
    pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
) -> core::DiagnosticResult<GeneratedPipelineOutput>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer,
    M: MetadataProvider,
    Q: QueryCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    let source_read = pipeline.source_reader.read(plan)?;
    let source_file_count = source_read.source_file_count();
    reject_mutations_until_pipeline_exists(&source_read)?;
    let (raw_queries, raw_fragments, mut diagnostics) = source_read.into_parts();
    let fragment_count = raw_fragments.len();
    let fragments_by_id = raw_fragments
        .iter()
        .map(|fragment| (fragment.metadata().id(), fragment))
        .collect::<HashMap<_, _>>();
    let mut compiled_queries = Vec::with_capacity(raw_queries.len());
    let mut query_summaries = Vec::with_capacity(raw_queries.len());
    let mut used_fragment_ids = HashSet::new();

    for query in &raw_queries {
        let analyzed_variants = analyze_query_variants(
            query,
            &fragments_by_id,
            &mut used_fragment_ids,
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
            validate_expanded_variant_param_bindings(
                variant,
                &metadata,
                &mut scoped_param_bindings,
            )
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
                &fragments_by_id,
                &scoped_param_bindings,
            )?)
        };
        query_summaries.push(QuerySummary::from_compiled_query(
            &compiled,
            unique_slot_count,
            variant_count,
        ));
        compiled_queries.push(compiled);
    }

    push_unused_fragment_warnings(&raw_fragments, &used_fragment_ids, &mut diagnostics);

    let generated_files = pipeline
        .target_generator
        .generate(plan, &compiled_queries)?;

    Ok(GeneratedPipelineOutput {
        generated_files,
        diagnostics,
        source_file_count,
        output_dir: plan.output_dir().to_path_buf(),
        query_summaries,
        fragment_count,
    })
}

fn reject_mutations_until_pipeline_exists(
    source_read: &crate::SourceRead,
) -> core::DiagnosticResult<()> {
    let Some(mutation) = source_read.mutations().first() else {
        return Ok(());
    };

    let mut diagnostic = core::Diagnostic::error(format!(
        "mutation source unit `{}` is parsed by source intake, but mutation analysis and generation are not implemented yet",
        mutation.metadata().id()
    ));
    if let Some(location) = mutation.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    Err(core::DiagnosticReport::new(diagnostic))
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
