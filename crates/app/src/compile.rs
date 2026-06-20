use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sqlcomp_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
    MetadataProvider, QueryCompiler, SourceReader, TargetGenerator,
};

const SLOT_VARIANT_LIMIT: usize = 256;

/// Application service for compile-like CLI commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompileUseCase;

/// Successful `check` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    fragment_count: usize,
}

impl CheckOutcome {
    /// Build a successful check outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        source_file_count: usize,
        output_dir: PathBuf,
        query_summaries: Vec<QuerySummary>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            fragment_count,
        }
    }

    /// Non-fatal diagnostics that should be shown to the user.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of SQL source files matched by source discovery.
    #[must_use]
    pub const fn source_file_count(&self) -> usize {
        self.source_file_count
    }

    /// Number of query blocks compiled.
    #[must_use]
    pub const fn query_count(&self) -> usize {
        self.query_summaries.len()
    }

    /// Generated output directory for this run.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Query-level summary data in source order.
    #[must_use]
    pub fn query_summaries(&self) -> &[QuerySummary] {
        &self.query_summaries
    }

    /// Number of global Fragment source units resolved in this run.
    #[must_use]
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// Number of unique query-local Slots resolved across compiled queries.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum()
    }

    /// Number of SQL variants validated across compiled queries.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::variant_count)
            .sum()
    }
}

/// Successful `compile` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    generated_file_paths: Vec<PathBuf>,
    stale_file_removal_count: Option<usize>,
    fragment_count: usize,
}

impl CompileOutcome {
    /// Build a successful compile outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        source_file_count: usize,
        output_dir: PathBuf,
        query_summaries: Vec<QuerySummary>,
        generated_file_paths: Vec<PathBuf>,
        stale_file_removal_count: Option<usize>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            generated_file_paths,
            stale_file_removal_count,
            fragment_count,
        }
    }

    /// Non-fatal diagnostics that should be shown to the user.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of SQL source files matched by source discovery.
    #[must_use]
    pub const fn source_file_count(&self) -> usize {
        self.source_file_count
    }

    /// Number of query blocks compiled.
    #[must_use]
    pub const fn query_count(&self) -> usize {
        self.query_summaries.len()
    }

    /// Generated output directory for this run.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Query-level summary data in source order.
    #[must_use]
    pub fn query_summaries(&self) -> &[QuerySummary] {
        &self.query_summaries
    }

    /// Number of generated files written or updated.
    #[must_use]
    pub const fn generated_file_count(&self) -> usize {
        self.generated_file_paths.len()
    }

    /// Generated file paths written or updated by this run.
    #[must_use]
    pub fn generated_file_paths(&self) -> &[PathBuf] {
        &self.generated_file_paths
    }

    /// Number of stale generated files removed when cleanup ran.
    #[must_use]
    pub const fn stale_file_removal_count(&self) -> Option<usize> {
        self.stale_file_removal_count
    }

    /// Number of global Fragment source units resolved in this run.
    #[must_use]
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// Number of unique query-local Slots resolved across compiled queries.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum()
    }

    /// Number of SQL variants validated across compiled queries.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::variant_count)
            .sum()
    }
}

/// Query-level success summary data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySummary {
    id: String,
    source_path: Option<PathBuf>,
    param_count: usize,
    input_field_count: usize,
    slot_count: usize,
    variant_count: usize,
}

impl QuerySummary {
    /// Build query-level summary data.
    #[must_use]
    pub const fn new(
        id: String,
        source_path: Option<PathBuf>,
        param_count: usize,
        input_field_count: usize,
        slot_count: usize,
        variant_count: usize,
    ) -> Self {
        Self {
            id,
            source_path,
            param_count,
            input_field_count,
            slot_count,
            variant_count,
        }
    }

    /// Query ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Number of generated parameter bindings, matching SQL placeholder occurrences.
    #[must_use]
    pub const fn param_count(&self) -> usize {
        self.param_count
    }

    /// Number of public input fields generated for this query.
    #[must_use]
    pub const fn input_field_count(&self) -> usize {
        self.input_field_count
    }

    /// Number of unique query-local Slots resolved for this query.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Number of SQL variants validated for this query.
    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.variant_count
    }

    fn from_compiled_query(
        query: &core::CompiledQuery,
        slot_count: usize,
        variant_count: usize,
    ) -> Self {
        Self::new(
            query.id().as_str().to_owned(),
            query.source_path().map(Path::to_path_buf),
            query.params().len(),
            query.input().len(),
            slot_count,
            variant_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_summary_counts_param_placeholders_and_input_fields_separately() {
        let query = core::CompiledQuery::new(
            core::QueryId::new("filterUsers".to_owned()),
            "SELECT id FROM users WHERE status = ? AND (email = ? OR email = ?);".to_owned(),
            core::Cardinality::Many,
            vec![
                core::InputField::new("status".to_owned(), core::CoreType::String, false),
                core::InputField::new("email".to_owned(), core::CoreType::String, false),
            ],
            Vec::new(),
        )
        .with_source_path("sql/users.sql")
        .with_params(vec![
            core::ParamBinding::new("status".to_owned(), core::CoreType::String, false),
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, false),
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, false),
        ]);

        let summary = QuerySummary::from_compiled_query(&query, 2, 6);

        assert_eq!(summary.id(), "filterUsers");
        assert_eq!(summary.source_path(), Some(Path::new("sql/users.sql")));
        assert_eq!(summary.param_count(), 3);
        assert_eq!(summary.input_field_count(), 2);
        assert_eq!(summary.slot_count(), 2);
        assert_eq!(summary.variant_count(), 6);
    }

    #[test]
    fn repeated_slot_fragment_param_validation_rejects_nullability_conflicts() {
        let first_slot_location = core::SourceLocation::at_position(
            "sql/users.sql",
            core::SourcePosition::one_based(8, 88).expect("test position should be valid"),
        );
        let second_slot_location = core::SourceLocation::at_position(
            "sql/users.sql",
            core::SourcePosition::one_based(9, 96).expect("test position should be valid"),
        );
        let query = core::RawQuery::new(
            core::QueryMetadata::new("listUsers".to_owned(), None),
            "SELECT id FROM users WHERE first = ? OR second = ?;".to_owned(),
        )
        .with_param_usages(vec![
            core::ParamUsage::new(
                "kind".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(35),
            core::ParamUsage::new(
                "kind".to_owned(),
                None,
                true,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(49),
        ]);
        let variant = AnalyzedQueryVariant {
            query,
            analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
            context: Some(SlotExpansionContext {
                query_id: "listUsers".to_owned(),
                selections: vec![SlotSelectionContext {
                    slot_id: "filter".to_owned(),
                    target_id: Some("byKind".to_owned()),
                    slot_location: first_slot_location.clone(),
                    fragment_location: None,
                }],
            }),
            param_scopes: vec![
                ExpandedParamScope::Fragment {
                    slot_id: "filter".to_owned(),
                    target_id: "byKind".to_owned(),
                },
                ExpandedParamScope::Fragment {
                    slot_id: "filter".to_owned(),
                    target_id: "byKind".to_owned(),
                },
            ],
            param_occurrences: vec![
                ExpandedParamOccurrence::Fragment(ExpandedFragmentParamOccurrence {
                    slot_id: "filter".to_owned(),
                    target_id: "byKind".to_owned(),
                    slot_occurrence_index: 1,
                    slot_location: first_slot_location,
                }),
                ExpandedParamOccurrence::Fragment(ExpandedFragmentParamOccurrence {
                    slot_id: "filter".to_owned(),
                    target_id: "byKind".to_owned(),
                    slot_occurrence_index: 2,
                    slot_location: second_slot_location,
                }),
            ],
        };
        let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
            core::DbParamUsage::new("kind".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("kind".to_owned(), core::CoreType::String),
        ]);
        let mut scoped_param_bindings = Vec::new();

        let report = validate_expanded_variant_param_bindings(
            &variant,
            &metadata,
            &mut scoped_param_bindings,
        )
        .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))
        .expect_err("repeated Slot Fragment Param nullability conflicts should be rejected");

        assert_eq!(
            report
                .diagnostics()
                .iter()
                .map(core::Diagnostic::message)
                .collect::<Vec<_>>()
                .join("\n"),
            "conflicting Fragment Param `kind` nullability in query `listUsers`, Slot `filter`, Fragment `byKind`: occurrence 1 is nullable false but occurrence 2 is nullable true; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability\nfirst occurrence of Slot `filter` selecting Fragment `byKind` is here\nconflicting occurrence of Slot `filter` selecting Fragment `byKind` is here\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=byKind\nSlot `filter` selected `byKind` in this variant"
        );
    }
}

/// Concrete port references required to run the compile pipeline.
#[derive(Clone, Copy, Debug)]
pub struct CompilePipeline<'a, P, S, D, M, Q, T, W>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer,
    M: MetadataProvider,
    Q: QueryCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    /// Compilation planner implementation.
    pub planner: &'a P,
    /// SQL source reader implementation.
    pub source_reader: &'a S,
    /// Dialect analyzer implementation.
    pub dialect_analyzer: &'a D,
    /// Database metadata provider implementation.
    pub metadata_provider: &'a M,
    /// Core IR compiler implementation.
    pub query_compiler: &'a Q,
    /// Target-language generator implementation.
    pub target_generator: &'a T,
    /// Generated file writer and cleaner implementation.
    pub generated_file_writer: &'a W,
}

impl DefaultCompileUseCase {
    /// Run the `check` command as a dry run of the full generation pipeline.
    ///
    /// Returns non-fatal diagnostics that should be shown to the user.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, core compilation, or target generation fails.
    pub fn check<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
    ) -> core::DiagnosticResult<CheckOutcome>
    where
        P: CompilationPlanner,
        S: SourceReader,
        D: DialectAnalyzer,
        M: MetadataProvider,
        Q: QueryCompiler,
        T: TargetGenerator,
        W: GeneratedFileWriter,
    {
        let plan = pipeline.planner.plan(config)?;
        let output = generate_files(&plan, pipeline)?;

        Ok(CheckOutcome::new(
            output.diagnostics,
            output.source_file_count,
            output.output_dir,
            output.query_summaries,
            output.fragment_count,
        ))
    }

    /// Run the `compile` command.
    ///
    /// Returns a success outcome with non-fatal diagnostics and write counts.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, generation, or file writing fails.
    pub fn compile<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        clean: bool,
    ) -> core::DiagnosticResult<CompileOutcome>
    where
        P: CompilationPlanner,
        S: SourceReader,
        D: DialectAnalyzer,
        M: MetadataProvider,
        Q: QueryCompiler,
        T: TargetGenerator,
        W: GeneratedFileWriter + GeneratedFileCleaner,
    {
        let plan = pipeline.planner.plan(config)?;

        let output = generate_files(&plan, pipeline)?;
        let generated_file_paths = output
            .generated_files
            .files()
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect::<Vec<_>>();
        pipeline
            .generated_file_writer
            .write(&output.generated_files)?;

        let stale_file_removal_count = if clean {
            Some(
                pipeline
                    .generated_file_writer
                    .clean_stale(plan.output_dir(), &output.generated_files)?,
            )
        } else {
            None
        };

        Ok(CompileOutcome::new(
            output.diagnostics,
            output.source_file_count,
            output.output_dir,
            output.query_summaries,
            generated_file_paths,
            stale_file_removal_count,
            output.fragment_count,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedPipelineOutput {
    generated_files: core::GeneratedFiles,
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    fragment_count: usize,
}

fn generate_files<P, S, D, M, Q, T, W>(
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
            validate_variant_row_shape(&base_metadata, variant, &metadata)
                .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct AnalyzedQueryVariants {
    variants: Vec<AnalyzedQueryVariant>,
    slot_specs: Vec<SlotSpec>,
    unique_slot_count: usize,
}

fn analyze_query_variants<D>(
    query: &core::RawQuery,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    dialect_analyzer: &D,
) -> core::DiagnosticResult<AnalyzedQueryVariants>
where
    D: DialectAnalyzer,
{
    if query.slot_usages().is_empty() {
        return Ok(AnalyzedQueryVariants {
            variants: vec![AnalyzedQueryVariant {
                query: query.clone(),
                analysis: dialect_analyzer.analyze(query)?,
                context: None,
                param_scopes: query
                    .param_usages()
                    .iter()
                    .map(|_| ExpandedParamScope::QueryDirect)
                    .collect(),
                param_occurrences: query
                    .param_usages()
                    .iter()
                    .map(|_| ExpandedParamOccurrence::QueryDirect)
                    .collect(),
            }],
            slot_specs: Vec::new(),
            unique_slot_count: 0,
        });
    }

    let slot_specs = unique_slot_specs(query)?;
    reject_direct_param_slot_collisions(query, &slot_specs)?;
    let variants = slot_validation_queries(query, &slot_specs, fragments_by_id, used_fragment_ids)?;
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

fn validate_variant_cardinality(variants: &[AnalyzedQueryVariant]) -> core::DiagnosticResult<()> {
    let Some(base_variant) = variants.first() else {
        return Ok(());
    };
    let base_cardinality = effective_cardinality(&base_variant.query, &base_variant.analysis);

    for variant in variants.iter().skip(1) {
        let variant_cardinality = effective_cardinality(&variant.query, &variant.analysis);
        if variant_cardinality != base_cardinality {
            return Err(with_slot_variant_context(
                query_error(
                    &variant.query,
                    format!(
                        "Slot expansion variant for query `{}` resolved effective cardinality `{}`, but the base variant resolved effective cardinality `{}`; all variants must have matching effective cardinality, using an explicit query metadata `cardinality` override when present and dialect analysis otherwise",
                        variant.query.metadata().id(),
                        format_cardinality(variant_cardinality),
                        format_cardinality(base_cardinality),
                    ),
                ),
                variant.context.as_ref(),
            ));
        }
    }

    Ok(())
}

fn validate_variant_row_shape(
    base_metadata: &core::DbQueryMetadata,
    variant: &AnalyzedQueryVariant,
    variant_metadata: &core::DbQueryMetadata,
) -> core::DiagnosticResult<()> {
    let base_columns = base_metadata.columns();
    let variant_columns = variant_metadata.columns();

    if variant_columns.len() != base_columns.len() {
        return Err(query_error(
            &variant.query,
            format!(
                "Slot expansion variant for query `{}` returned {} result columns, but the base variant returned {}; all variants must have matching result row shape",
                variant.query.metadata().id(),
                variant_columns.len(),
                base_columns.len(),
            ),
        ));
    }

    for (index, (base_column, variant_column)) in
        base_columns.iter().zip(variant_columns).enumerate()
    {
        let column_number = index + 1;
        if variant_column.name() != base_column.name() {
            let difference = format!(
                "result column {column_number} name `{}` does not match base column name `{}`",
                variant_column.name(),
                base_column.name(),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
        if variant_column.ty() != base_column.ty() {
            let difference = format!(
                "result column {column_number} CoreType `{:?}` does not match base CoreType `{:?}`",
                variant_column.ty(),
                base_column.ty(),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
        if variant_column.is_nullable_for_output() != base_column.is_nullable_for_output() {
            let difference = format!(
                "result column {column_number} nullability `{}` does not match base nullability `{}`",
                format_nullability(variant_column.is_nullable_for_output()),
                format_nullability(base_column.is_nullable_for_output()),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
    }

    Ok(())
}

fn row_shape_difference_error(
    variant: &AnalyzedQueryVariant,
    difference: &str,
) -> core::DiagnosticReport {
    query_error(
        &variant.query,
        format!(
            "Slot expansion variant for query `{}` {difference}; all variants must have matching result row shape",
            variant.query.metadata().id(),
        ),
    )
}

fn effective_cardinality(
    query: &core::RawQuery,
    analysis: &core::AnalyzedQuery,
) -> core::Cardinality {
    query
        .metadata()
        .cardinality()
        .unwrap_or_else(|| analysis.cardinality())
}

const fn format_nullability(nullable: bool) -> &'static str {
    if nullable { "nullable" } else { "not nullable" }
}

const fn format_cardinality(cardinality: core::Cardinality) -> &'static str {
    match cardinality {
        core::Cardinality::One => "one",
        core::Cardinality::Many => "many",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AnalyzedQueryVariant {
    query: core::RawQuery,
    analysis: core::AnalyzedQuery,
    context: Option<SlotExpansionContext>,
    param_scopes: Vec<ExpandedParamScope>,
    param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotSpec {
    id: String,
    targets: Vec<String>,
    source_location: core::SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotExpansionVariant {
    query: core::RawQuery,
    context: SlotExpansionContext,
    param_scopes: Vec<ExpandedParamScope>,
    param_occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExpandedParamScope {
    QueryDirect,
    Fragment { slot_id: String, target_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExpandedParamOccurrence {
    QueryDirect,
    Fragment(ExpandedFragmentParamOccurrence),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpandedFragmentParamOccurrence {
    slot_id: String,
    target_id: String,
    slot_occurrence_index: usize,
    slot_location: core::SourceLocation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ExpandedParamBuffers {
    usages: Vec<core::ParamUsage>,
    scopes: Vec<ExpandedParamScope>,
    occurrences: Vec<ExpandedParamOccurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScopedParamBinding {
    scope: ExpandedParamScope,
    id: String,
    ty: core::CoreType,
    nullable: bool,
    first_occurrence: ExpandedParamOccurrence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotExpansionContext {
    query_id: String,
    selections: Vec<SlotSelectionContext>,
}

impl SlotExpansionContext {
    fn diagnostics(&self) -> Vec<core::Diagnostic> {
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
            "while validating Slot expansion variant for query `{}` with selections: {selection_summary}",
            self.query_id
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotSelectionContext {
    slot_id: String,
    target_id: Option<String>,
    slot_location: core::SourceLocation,
    fragment_location: Option<core::SourceLocation>,
}

fn slot_validation_queries(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
) -> core::DiagnosticResult<Vec<SlotExpansionVariant>> {
    let variant_choices =
        slot_variant_choices(query, slot_specs, fragments_by_id, used_fragment_ids)?;

    variant_choices
        .iter()
        .map(|choices| build_slot_variant_query(query, slot_specs, choices))
        .collect()
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
        context: slot_expansion_context(query, slot_specs, choices),
        param_scopes: params.scopes,
        param_occurrences: params.occurrences,
    })
}

fn compile_dynamic_query_body(
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

fn slot_expansion_context(
    query: &core::RawQuery,
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
        query_id: query.metadata().id().to_owned(),
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

fn validate_expanded_variant_param_bindings(
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

fn query_param_placeholder_index(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<usize> {
    usage.placeholder_index().ok_or_else(|| {
        param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` in query `{}` is missing placeholder position metadata",
                usage.id(),
                query.metadata().id()
            ),
        )
    })
}

fn with_slot_variant_context(
    report: core::DiagnosticReport,
    context: Option<&SlotExpansionContext>,
) -> core::DiagnosticReport {
    let Some(context) = context else {
        return report;
    };

    let mut diagnostics = report.into_diagnostics();
    diagnostics.extend(context.diagnostics());
    core::DiagnosticReport::from_diagnostics(diagnostics)
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

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

fn slot_usage_error(
    query: &core::RawQuery,
    usage: &core::SlotUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

fn param_usage_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

fn location_error(
    location: core::SourceLocation,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(message).with_location(location))
}

/// Dummy port bundle showing dependencies required by compile-like use cases.
pub trait CompileUseCasePorts {
    /// Configuration loader implementation.
    type ConfigLoader: crate::ConfigLoader;

    /// Compilation planner implementation.
    type CompilationPlanner: CompilationPlanner;

    /// Source reader implementation.
    type SourceReader: SourceReader;

    /// Dialect analyzer implementation.
    type DialectAnalyzer: DialectAnalyzer;

    /// Metadata provider implementation.
    type MetadataProvider: MetadataProvider;

    /// Query compiler implementation.
    type QueryCompiler: QueryCompiler;

    /// Target generator implementation.
    type TargetGenerator: TargetGenerator;

    /// Generated file writer and cleaner implementation.
    type GeneratedFileWriter: GeneratedFileWriter + GeneratedFileCleaner;
}
