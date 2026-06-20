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
}

impl CheckOutcome {
    /// Build a successful check outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        source_file_count: usize,
        output_dir: PathBuf,
        query_summaries: Vec<QuerySummary>,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
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
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            generated_file_paths,
            stale_file_removal_count,
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
}

/// Query-level success summary data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySummary {
    id: String,
    source_path: Option<PathBuf>,
    param_count: usize,
    input_field_count: usize,
}

impl QuerySummary {
    /// Build query-level summary data.
    #[must_use]
    pub const fn new(
        id: String,
        source_path: Option<PathBuf>,
        param_count: usize,
        input_field_count: usize,
    ) -> Self {
        Self {
            id,
            source_path,
            param_count,
            input_field_count,
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

    fn from_compiled_query(query: &core::CompiledQuery) -> Self {
        Self::new(
            query.id().as_str().to_owned(),
            query.source_path().map(Path::to_path_buf),
            query.params().len(),
            query.input().len(),
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

        let summary = QuerySummary::from_compiled_query(&query);

        assert_eq!(summary.id(), "filterUsers");
        assert_eq!(summary.source_path(), Some(Path::new("sql/users.sql")));
        assert_eq!(summary.param_count(), 3);
        assert_eq!(summary.input_field_count(), 2);
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
    let fragments_by_id = raw_fragments
        .iter()
        .map(|fragment| (fragment.metadata().id(), fragment))
        .collect::<HashMap<_, _>>();
    let mut compiled_queries = Vec::with_capacity(raw_queries.len());
    let mut used_fragment_ids = HashSet::new();

    for query in &raw_queries {
        let analyzed_variants = analyze_query_variants(
            query,
            &fragments_by_id,
            &mut used_fragment_ids,
            pipeline.dialect_analyzer,
        )?;
        let Some(base_variant) = analyzed_variants.first() else {
            return Err(query_error(
                query,
                "Slot expansion produced no validation variants",
            ));
        };
        validate_variant_cardinality(&analyzed_variants)?;
        let base_metadata = pipeline
            .metadata_provider
            .describe(&base_variant.query, &base_variant.analysis)
            .map_err(|report| with_slot_variant_context(report, base_variant.context.as_ref()))?;
        for variant in analyzed_variants.iter().skip(1) {
            let metadata = pipeline
                .metadata_provider
                .describe(&variant.query, &variant.analysis)
                .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
            validate_variant_row_shape(&base_metadata, variant, &metadata)?;
            crate::query_compiler::validate_param_bindings(&variant.query, &metadata)
                .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))?;
        }
        let compiled = pipeline.query_compiler.compile(
            &base_variant.query,
            &base_variant.analysis,
            &base_metadata,
        )?;
        compiled_queries.push(compiled);
    }

    push_unused_fragment_warnings(&raw_fragments, &used_fragment_ids, &mut diagnostics);

    let generated_files = pipeline
        .target_generator
        .generate(plan, &compiled_queries)?;
    let query_summaries = compiled_queries
        .iter()
        .map(QuerySummary::from_compiled_query)
        .collect();

    Ok(GeneratedPipelineOutput {
        generated_files,
        diagnostics,
        source_file_count,
        output_dir: plan.output_dir().to_path_buf(),
        query_summaries,
    })
}

fn analyze_query_variants<D>(
    query: &core::RawQuery,
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
    dialect_analyzer: &D,
) -> core::DiagnosticResult<Vec<AnalyzedQueryVariant>>
where
    D: DialectAnalyzer,
{
    if query.slot_usages().is_empty() {
        return Ok(vec![AnalyzedQueryVariant {
            query: query.clone(),
            analysis: dialect_analyzer.analyze(query)?,
            context: None,
        }]);
    }

    let variants = slot_validation_queries(query, fragments_by_id, used_fragment_ids)?;
    let mut analyzed_variants = Vec::with_capacity(variants.len());
    for variant in variants {
        let analysis = dialect_analyzer
            .analyze(&variant.query)
            .map_err(|report| with_slot_variant_context(report, Some(&variant.context)))?;
        analyzed_variants.push(AnalyzedQueryVariant {
            query: variant.query,
            analysis,
            context: Some(variant.context),
        });
    }

    Ok(analyzed_variants)
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
                        "Slot expansion variant for query `{}` resolved cardinality `{}`, but the base variant resolved cardinality `{}`; all variants must have matching cardinality after query metadata override",
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
        return Err(with_slot_variant_context(
            query_error(
                &variant.query,
                format!(
                    "Slot expansion variant for query `{}` returned {} result columns, but the base variant returned {}; all variants must have matching result row shape",
                    variant.query.metadata().id(),
                    variant_columns.len(),
                    base_columns.len(),
                ),
            ),
            variant.context.as_ref(),
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
    with_slot_variant_context(
        query_error(
            &variant.query,
            format!(
                "Slot expansion variant for query `{}` {difference}; all variants must have matching result row shape",
                variant.query.metadata().id(),
            ),
        ),
        variant.context.as_ref(),
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
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
    used_fragment_ids: &mut HashSet<String>,
) -> core::DiagnosticResult<Vec<SlotExpansionVariant>> {
    let slot_specs = unique_slot_specs(query)?;
    reject_direct_param_slot_collisions(query, &slot_specs)?;
    let variant_choices =
        slot_variant_choices(query, &slot_specs, fragments_by_id, used_fragment_ids)?;

    variant_choices
        .iter()
        .map(|choices| build_slot_variant_query(query, &slot_specs, choices))
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
    let mut param_usages = Vec::new();

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
            &mut param_usages,
        )?;
        if let Some(Some(fragment)) = choices_by_slot.get(usage.id()) {
            let fragment_output_start = analysis_sql.len();
            analysis_sql.push_str(fragment.analysis_sql());
            push_fragment_params(fragment, fragment_output_start, &mut param_usages, query)?;
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
        &mut param_usages,
    )?;

    let mut expanded_query = core::RawQuery::new(query.metadata().clone(), query.sql().to_owned())
        .with_analysis_sql(analysis_sql)
        .with_param_usages(param_usages);

    if let Some(source_path) = query.source_path() {
        expanded_query = expanded_query.with_source_path(source_path.to_path_buf());
    }
    if let Some(source_location) = query.source_location() {
        expanded_query = expanded_query.with_source_location(source_location.clone());
    }

    Ok(SlotExpansionVariant {
        query: expanded_query,
        context: slot_expansion_context(query, slot_specs, choices),
    })
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
    param_usages: &mut Vec<core::ParamUsage>,
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

        param_usages.push(
            usage
                .clone()
                .with_placeholder_index(segment_output_start + placeholder_index - segment_start),
        );
        *query_param_cursor += 1;
    }

    Ok(())
}

fn push_fragment_params(
    fragment: &core::RawFragment,
    fragment_output_start: usize,
    param_usages: &mut Vec<core::ParamUsage>,
    query: &core::RawQuery,
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

        param_usages.push(
            usage
                .clone()
                .with_placeholder_index(fragment_output_start + placeholder_index),
        );
    }

    Ok(())
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
