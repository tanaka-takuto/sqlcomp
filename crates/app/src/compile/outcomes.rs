use std::path::{Path, PathBuf};

use sqlay_core as core;

/// Successful `check` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    mutation_summaries: Vec<MutationSummary>,
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
        mutation_summaries: Vec<MutationSummary>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            mutation_summaries,
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

    /// Number of mutation blocks compiled.
    #[must_use]
    pub const fn mutation_count(&self) -> usize {
        self.mutation_summaries.len()
    }

    /// Number of query and mutation builders compiled.
    #[must_use]
    pub const fn builder_count(&self) -> usize {
        self.query_count() + self.mutation_count()
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

    /// Mutation-level summary data in source order.
    #[must_use]
    pub fn mutation_summaries(&self) -> &[MutationSummary] {
        &self.mutation_summaries
    }

    /// Number of global Fragment source units resolved in this run.
    #[must_use]
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// Number of unique Slots resolved across compiled builders.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::slot_count)
                .sum::<usize>()
    }

    /// Number of unique Repeats resolved across compiled builders.
    #[must_use]
    pub fn unique_repeat_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::repeat_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::repeat_count)
                .sum::<usize>()
    }

    /// Number of SQL validation cases checked across compiled builders.
    #[must_use]
    pub fn validation_case_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::validation_case_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::validation_case_count)
                .sum::<usize>()
    }

    /// Number of SQL validation cases checked across compiled builders.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.validation_case_count()
    }
}

/// Successful `compile` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    mutation_summaries: Vec<MutationSummary>,
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
        mutation_summaries: Vec<MutationSummary>,
        generated_file_paths: Vec<PathBuf>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            mutation_summaries,
            generated_file_paths,
            stale_file_removal_count: None,
            fragment_count,
        }
    }

    /// Attach the stale file cleanup count when `compile --clean` ran.
    #[must_use]
    pub const fn with_stale_file_removal_count(
        mut self,
        stale_file_removal_count: Option<usize>,
    ) -> Self {
        self.stale_file_removal_count = stale_file_removal_count;
        self
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

    /// Number of mutation blocks compiled.
    #[must_use]
    pub const fn mutation_count(&self) -> usize {
        self.mutation_summaries.len()
    }

    /// Number of query and mutation builders compiled.
    #[must_use]
    pub const fn builder_count(&self) -> usize {
        self.query_count() + self.mutation_count()
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

    /// Mutation-level summary data in source order.
    #[must_use]
    pub fn mutation_summaries(&self) -> &[MutationSummary] {
        &self.mutation_summaries
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

    /// Number of unique Slots resolved across compiled builders.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::slot_count)
                .sum::<usize>()
    }

    /// Number of unique Repeats resolved across compiled builders.
    #[must_use]
    pub fn unique_repeat_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::repeat_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::repeat_count)
                .sum::<usize>()
    }

    /// Number of SQL validation cases checked across compiled builders.
    #[must_use]
    pub fn validation_case_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::validation_case_count)
            .sum::<usize>()
            + self
                .mutation_summaries
                .iter()
                .map(MutationSummary::validation_case_count)
                .sum::<usize>()
    }

    /// Number of SQL validation cases checked across compiled builders.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.validation_case_count()
    }
}

/// Shared builder-level summary counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuilderSummaryCounts {
    params: usize,
    input_fields: usize,
    slots: usize,
    repeats: usize,
    validation_cases: usize,
}

impl BuilderSummaryCounts {
    /// Build shared builder-level summary counts.
    #[must_use]
    pub const fn new(
        param_count: usize,
        input_field_count: usize,
        slot_count: usize,
        repeat_count: usize,
        validation_case_count: usize,
    ) -> Self {
        Self {
            params: param_count,
            input_fields: input_field_count,
            slots: slot_count,
            repeats: repeat_count,
            validation_cases: validation_case_count,
        }
    }

    /// Number of generated parameter bindings, matching SQL placeholder occurrences.
    #[must_use]
    pub const fn param_count(&self) -> usize {
        self.params
    }

    /// Number of public input fields generated for this query.
    #[must_use]
    pub const fn input_field_count(&self) -> usize {
        self.input_fields
    }

    /// Number of unique query-local Slots resolved for this query.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.slots
    }

    /// Number of unique Repeats resolved for this query.
    #[must_use]
    pub const fn repeat_count(&self) -> usize {
        self.repeats
    }

    /// Number of SQL validation cases checked for this query.
    #[must_use]
    pub const fn validation_case_count(&self) -> usize {
        self.validation_cases
    }

    /// Number of SQL validation cases checked for this query.
    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.validation_case_count()
    }
}

/// Query-level success summary data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySummary {
    id: String,
    source_path: Option<PathBuf>,
    counts: BuilderSummaryCounts,
}

impl QuerySummary {
    /// Build query-level summary data.
    #[must_use]
    pub const fn new(
        id: String,
        source_path: Option<PathBuf>,
        counts: BuilderSummaryCounts,
    ) -> Self {
        Self {
            id,
            source_path,
            counts,
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
        self.counts.param_count()
    }

    /// Number of public input fields generated for this query.
    #[must_use]
    pub const fn input_field_count(&self) -> usize {
        self.counts.input_field_count()
    }

    /// Number of unique query-local Slots resolved for this query.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.counts.slot_count()
    }

    /// Number of unique Repeats resolved for this query.
    #[must_use]
    pub const fn repeat_count(&self) -> usize {
        self.counts.repeat_count()
    }

    /// Number of SQL validation cases checked for this query.
    #[must_use]
    pub const fn validation_case_count(&self) -> usize {
        self.counts.validation_case_count()
    }

    /// Number of SQL validation cases checked for this query.
    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.counts.variant_count()
    }

    pub(super) fn from_compiled_query(
        query: &core::CompiledQuery,
        slot_count: usize,
        repeat_count: usize,
        validation_case_count: usize,
    ) -> Self {
        Self::new(
            query.id().as_str().to_owned(),
            query.source_path().map(Path::to_path_buf),
            BuilderSummaryCounts::new(
                query.params().len(),
                query.input().len(),
                slot_count,
                repeat_count,
                validation_case_count,
            ),
        )
    }
}

/// Mutation-level success summary data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationSummary {
    id: String,
    source_path: Option<PathBuf>,
    kind: core::MutationKind,
    counts: BuilderSummaryCounts,
}

impl MutationSummary {
    /// Build mutation-level summary data.
    #[must_use]
    pub const fn new(
        id: String,
        source_path: Option<PathBuf>,
        kind: core::MutationKind,
        counts: BuilderSummaryCounts,
    ) -> Self {
        Self {
            id,
            source_path,
            kind,
            counts,
        }
    }

    /// Mutation ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Supported mutation statement family.
    #[must_use]
    pub const fn kind(&self) -> core::MutationKind {
        self.kind
    }

    /// Number of generated parameter bindings, matching SQL placeholder occurrences.
    #[must_use]
    pub const fn param_count(&self) -> usize {
        self.counts.param_count()
    }

    /// Number of public input fields generated for this mutation.
    #[must_use]
    pub const fn input_field_count(&self) -> usize {
        self.counts.input_field_count()
    }

    /// Number of unique mutation-local Slots resolved for this mutation.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.counts.slot_count()
    }

    /// Number of unique Repeats resolved for this mutation.
    #[must_use]
    pub const fn repeat_count(&self) -> usize {
        self.counts.repeat_count()
    }

    /// Number of SQL validation cases checked for this mutation.
    #[must_use]
    pub const fn validation_case_count(&self) -> usize {
        self.counts.validation_case_count()
    }

    /// Number of SQL validation cases checked for this mutation.
    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.counts.variant_count()
    }

    pub(super) fn from_compiled_mutation(
        mutation: &core::CompiledMutation,
        slot_count: usize,
        repeat_count: usize,
        validation_case_count: usize,
    ) -> Self {
        Self::new(
            mutation.id().as_str().to_owned(),
            mutation.source_path().map(Path::to_path_buf),
            mutation.kind(),
            BuilderSummaryCounts::new(
                mutation.params().len(),
                mutation.input().len(),
                slot_count,
                repeat_count,
                validation_case_count,
            ),
        )
    }
}
