use super::*;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn project_config(config_dir: PathBuf) -> core::ProjectConfig {
    core::ProjectConfig::new(
        config_dir,
        core::SourceConfig::new(
            vec!["sql/**/*.sql".to_owned()],
            vec!["sql/private/**/*.sql".to_owned()],
        ),
        core::OutputConfig::new("src/generated/sqlay".to_owned()),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    )
}

pub(super) fn compile_query(
    explicit_cardinality: Option<core::Cardinality>,
    inferred_cardinality: core::Cardinality,
) -> core::CompiledQuery {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), explicit_cardinality),
        "SELECT id FROM users;".to_owned(),
    );
    let analysis = core::AnalyzedQuery::new(inferred_cardinality);

    DefaultQueryCompiler
        .compile(&query, &analysis, &core::DbQueryMetadata::new(Vec::new()))
        .expect("query compiler should resolve cardinality")
}

pub(super) fn slot_param_order_fixture() -> (core::RawQuery, core::RawFragment) {
    let base_sql = "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1 AND u.email = ?;";
    let slot_index = base_sql
        .find(" AND u.email")
        .expect("Slot insertion point exists before email predicate");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1/* @sqlay { type: slot id: filter targets: [activeAndRole] } */ AND u.email = ?;"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "tenantId",
            base_sql
                .find('?')
                .expect("tenant Param placeholder exists"),
        ),
        test_param_usage("email", base_sql.rfind('?').expect("email Param placeholder exists")),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeAndRole".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");

    let fragment_sql = "\nAND u.active = ? AND u.role_id = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeAndRole".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "active",
            fragment_sql
                .find('?')
                .expect("active Param placeholder exists"),
        ),
        test_param_usage(
            "roleId",
            fragment_sql
                .rfind('?')
                .expect("role Param placeholder exists"),
        ),
    ])
    .with_source_path("sql/fragments.sql");

    (query, fragment)
}

pub(super) fn repeated_slot_dynamic_ir_fixture() -> (core::RawQuery, core::RawFragment) {
    let base_sql = "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1 AND EXISTS (SELECT 1 FROM users AS ux WHERE ux.id = u.id);";
    let first_slot_index = base_sql
        .find(" AND EXISTS")
        .expect("first Slot insertion point exists before EXISTS predicate");
    let second_slot_index = base_sql
        .find(");")
        .expect("second Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1/* @sqlay { type: slot id: filter targets: [activeOnly] } */ AND EXISTS (SELECT 1 FROM users AS ux WHERE ux.id = u.id/* @sqlay { type: slot id: filter targets: [activeOnly] } */);"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "tenantId",
        base_sql
            .find('?')
            .expect("tenant Param placeholder exists"),
    )])
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["activeOnly".to_owned()],
            first_slot_index,
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["activeOnly".to_owned()],
            second_slot_index,
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let fragment_sql = "\nAND u.active = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "active",
        fragment_sql
            .find('?')
            .expect("active Param placeholder exists"),
    )])
    .with_source_path("sql/fragments.sql");

    (query, fragment)
}

pub(super) fn row_shape_mismatch_report(
    variant_columns: Vec<core::DbResultColumn>,
) -> core::DiagnosticReport {
    row_shape_mismatch_report_with_base(
        vec![core::DbResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            Some(false),
        )],
        variant_columns,
    )
}

pub(super) fn row_shape_mismatch_report_with_base(
    base_columns: Vec<core::DbResultColumn>,
    variant_columns: Vec<core::DbResultColumn>,
) -> core::DiagnosticReport {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let query_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = query_sql
        .find(';')
        .expect("Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [shapeChanger] } */;".to_owned(),
    )
    .with_analysis_sql(query_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["shapeChanger".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("shapeChanger".to_owned()),
        "\nAND u.email IS NOT NULL".to_owned(),
    )
    .with_analysis_sql("\nAND u.email IS NOT NULL".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_columns_for_sql(query_sql, base_columns)
        .with_columns_for_sql("\nAND u.email IS NOT NULL", variant_columns);
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator =
        FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()));
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls);
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("row-shape-changing Slot variants should be rejected")
}

pub(super) fn test_param_usage(id: &str, placeholder_index: usize) -> core::ParamUsage {
    core::ParamUsage::new(id.to_owned(), None, false, core::SourceLocation::unknown())
        .with_placeholder_index(placeholder_index)
}

#[test]
fn fake_dialect_analyzer_matches_limit_one_case_insensitively_and_exactly() {
    let analyzer = FakeDialectAnalyzer::new(CallLog::default()).with_limit_one_inference();

    let limit_one = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOne".to_owned(), None),
            "SELECT id FROM users limit 1;".to_owned(),
        ))
        .expect("lowercase limit one should be analyzed");
    let limit_ten = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitTen".to_owned(), None),
            "SELECT id FROM users LIMIT 10;".to_owned(),
        ))
        .expect("limit ten should be analyzed");
    let limit_one_offset = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOneOffset".to_owned(), None),
            "SELECT id FROM users LIMIT 1 OFFSET 5;".to_owned(),
        ))
        .expect("limit one with additional clause should be analyzed");
    let limit_one_lowercase_offset = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOneLowercaseOffset".to_owned(), None),
            "SELECT id FROM users LIMIT 1 offset 5;".to_owned(),
        ))
        .expect("limit one with lowercase offset should be analyzed");
    let limit_one_mixed_case_offset = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOneMixedCaseOffset".to_owned(), None),
            "SELECT id FROM users LIMIT 1 OffSeT 5;".to_owned(),
        ))
        .expect("limit one with mixed-case offset should be analyzed");

    assert_eq!(limit_one.cardinality(), core::Cardinality::One);
    assert_eq!(limit_ten.cardinality(), core::Cardinality::Many);
    assert_eq!(limit_one_offset.cardinality(), core::Cardinality::One);
    assert_eq!(
        limit_one_lowercase_offset.cardinality(),
        core::Cardinality::One
    );
    assert_eq!(
        limit_one_mixed_case_offset.cardinality(),
        core::Cardinality::One
    );
}

pub(super) fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn raw_query() -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users;".to_owned(),
    )
    .with_source_path("sql/users.sql")
}

pub(super) fn metadata() -> core::DbQueryMetadata {
    core::DbQueryMetadata::new(vec![core::DbResultColumn::new(
        "id".to_owned(),
        core::CoreType::Int64,
        Some(false),
    )])
}

pub(super) fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

#[derive(Clone, Debug, Default)]
pub(super) struct CallLog(Rc<RefCell<Vec<&'static str>>>);

impl CallLog {
    fn push(&self, call: &'static str) {
        self.0.borrow_mut().push(call);
    }

    pub(super) fn entries(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PipelineFailure {
    Dialect,
    Metadata,
    Generation,
}

impl PipelineFailure {
    pub(super) const fn message(self) -> &'static str {
        match self {
            Self::Dialect => "dialect failed",
            Self::Metadata => "metadata failed",
            Self::Generation => "generation failed",
        }
    }

    fn report(self) -> core::DiagnosticReport {
        core::DiagnosticReport::new(core::Diagnostic::error(self.message()))
    }
}

#[derive(Clone, Debug)]
pub(super) struct FakeSourceReader {
    calls: CallLog,
    source_read: SourceRead,
}

impl FakeSourceReader {
    pub(super) fn new(calls: CallLog) -> Self {
        Self {
            calls,
            source_read: SourceRead::from_queries(vec![raw_query()]).with_source_file_count(1),
        }
    }

    pub(super) fn with_source_read(mut self, source_read: SourceRead) -> Self {
        self.source_read = source_read;
        self
    }
}

impl SourceReader for FakeSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        self.calls.push("read");

        Ok(self.source_read.clone())
    }
}

#[derive(Clone, Debug)]
pub(super) struct FakeDialectAnalyzer {
    calls: CallLog,
    failure: Option<PipelineFailure>,
    sql_failure: Option<&'static str>,
    infer_limit_one: bool,
    analyzed_sql: Rc<RefCell<Vec<String>>>,
}

impl FakeDialectAnalyzer {
    pub(super) fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
            sql_failure: None,
            infer_limit_one: false,
            analyzed_sql: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(super) fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Dialect) {
            self.failure = Some(failure);
        }

        self
    }

    pub(super) const fn with_sql_failure(mut self, sql_fragment: &'static str) -> Self {
        self.sql_failure = Some(sql_fragment);
        self
    }

    pub(super) const fn with_limit_one_inference(mut self) -> Self {
        self.infer_limit_one = true;
        self
    }

    pub(super) fn analyzed_sql(&self) -> Vec<String> {
        self.analyzed_sql.borrow().clone()
    }
}

impl DialectAnalyzer for FakeDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        self.calls.push("analyze");
        self.analyzed_sql
            .borrow_mut()
            .push(query.analysis_sql().to_owned());

        if self
            .sql_failure
            .is_some_and(|sql_fragment| query.analysis_sql().contains(sql_fragment))
        {
            return Err(core::DiagnosticReport::new(core::Diagnostic::error(
                "failed to parse MySQL SQL: token-adjacent slot replacement",
            )));
        }

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        let cardinality =
            if self.infer_limit_one && analysis_sql_has_limit_one(query.analysis_sql()) {
                core::Cardinality::One
            } else {
                core::Cardinality::Many
            };

        Ok(core::AnalyzedQuery::new(cardinality))
    }
}

impl MutationAnalyzer for FakeDialectAnalyzer {
    fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation> {
        self.calls.push("analyze_mutation");
        self.analyzed_sql
            .borrow_mut()
            .push(mutation.analysis_sql().to_owned());

        if self
            .sql_failure
            .is_some_and(|sql_fragment| mutation.analysis_sql().contains(sql_fragment))
        {
            return Err(core::DiagnosticReport::new(core::Diagnostic::error(
                "failed to parse MySQL SQL: token-adjacent slot replacement",
            )));
        }

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        Ok(core::AnalyzedMutation::new(infer_mutation_kind(
            mutation.analysis_sql(),
        )))
    }
}

fn infer_mutation_kind(sql: &str) -> core::MutationKind {
    let trimmed = sql.trim_start();

    if trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("INSERT"))
    {
        core::MutationKind::Insert
    } else if trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UPDATE"))
    {
        core::MutationKind::Update
    } else if trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("DELETE"))
    {
        core::MutationKind::Delete
    } else if trimmed
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("REPLACE"))
    {
        core::MutationKind::Replace
    } else {
        panic!("unexpected mutation SQL in test fake: {trimmed}");
    }
}

fn analysis_sql_has_limit_one(sql: &str) -> bool {
    let tokens = sql.split_ascii_whitespace().collect::<Vec<_>>();

    for index in 0..tokens.len().saturating_sub(1) {
        if !tokens[index].eq_ignore_ascii_case("LIMIT") {
            continue;
        }

        match tokens[index + 1] {
            "1;" => {
                return index + 2 == tokens.len();
            }
            "1" => {
                return index + 2 == tokens.len()
                    || (tokens.get(index + 2) == Some(&";") && index + 3 == tokens.len())
                    || tokens
                        .get(index + 2)
                        .is_some_and(|token| token.eq_ignore_ascii_case("OFFSET"));
            }
            _ => {}
        }
    }

    false
}

#[derive(Clone, Debug)]
pub(super) struct FakeMetadataProvider {
    calls: CallLog,
    failure: Option<PipelineFailure>,
    param_failure: Option<(&'static str, &'static str)>,
    columns_by_sql_fragment: Vec<(&'static str, Vec<core::DbResultColumn>)>,
    param_types_by_placeholder_prefix: Vec<(&'static str, core::CoreType)>,
    described_sql: Rc<RefCell<Vec<String>>>,
    described_param_ids: Rc<RefCell<Vec<Vec<String>>>>,
}

impl FakeMetadataProvider {
    pub(super) fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
            param_failure: None,
            columns_by_sql_fragment: Vec::new(),
            param_types_by_placeholder_prefix: Vec::new(),
            described_sql: Rc::new(RefCell::new(Vec::new())),
            described_param_ids: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(super) const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Metadata) {
            self.failure = Some(failure);
        }

        self
    }

    fn with_columns_for_sql(
        mut self,
        sql_fragment: &'static str,
        columns: Vec<core::DbResultColumn>,
    ) -> Self {
        self.columns_by_sql_fragment.push((sql_fragment, columns));
        self
    }

    pub(super) const fn with_param_failure(
        mut self,
        id: &'static str,
        message: &'static str,
    ) -> Self {
        self.param_failure = Some((id, message));
        self
    }

    pub(super) fn with_param_type_before_placeholder(
        mut self,
        sql_prefix: &'static str,
        ty: core::CoreType,
    ) -> Self {
        self.param_types_by_placeholder_prefix
            .push((sql_prefix, ty));
        self
    }

    pub(super) fn described_sql(&self) -> Vec<String> {
        self.described_sql.borrow().clone()
    }

    pub(super) fn described_param_ids(&self) -> Vec<Vec<String>> {
        self.described_param_ids.borrow().clone()
    }
}

impl MetadataProvider for FakeMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        self.calls.push("describe");
        self.described_sql
            .borrow_mut()
            .push(query.analysis_sql().to_owned());
        let param_ids = query
            .param_usages()
            .iter()
            .map(|usage| usage.id().to_owned())
            .collect::<Vec<_>>();
        self.described_param_ids.borrow_mut().push(param_ids);

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }
        if let Some((id, message)) = self.param_failure
            && let Some(usage) = query.param_usages().iter().find(|usage| usage.id() == id)
        {
            return Err(core::DiagnosticReport::new(
                core::Diagnostic::error(message).with_location(usage.source_location().clone()),
            ));
        }

        let param_usages = query
            .param_usages()
            .iter()
            .map(|usage| {
                core::DbParamUsage::new(
                    usage.id().to_owned(),
                    self.param_type_for_usage(query, usage).unwrap_or_else(|| {
                        usage
                            .value_type_override()
                            .unwrap_or(core::CoreType::String)
                    }),
                )
            })
            .collect();

        let columns = self
            .columns_by_sql_fragment
            .iter()
            .find(|(sql_fragment, _)| query.analysis_sql().contains(sql_fragment))
            .map_or_else(
                || metadata().columns().to_vec(),
                |(_, columns)| columns.clone(),
            );

        Ok(core::DbQueryMetadata::new(columns).with_param_usages(param_usages))
    }
}

impl MutationMetadataProvider for FakeMetadataProvider {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        _analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        self.calls.push("describe_mutation");
        self.described_sql
            .borrow_mut()
            .push(mutation.analysis_sql().to_owned());
        let param_ids = mutation
            .param_usages()
            .iter()
            .map(|usage| usage.id().to_owned())
            .collect::<Vec<_>>();
        self.described_param_ids.borrow_mut().push(param_ids);

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }
        if let Some((id, message)) = self.param_failure {
            let Some(usage) = mutation
                .param_usages()
                .iter()
                .find(|usage| usage.id() == id)
            else {
                return Err(core::DiagnosticReport::new(core::Diagnostic::error(
                    format!("test fake mutation Param failure targets unknown Param id `{id}`"),
                )));
            };

            return Err(core::DiagnosticReport::new(
                core::Diagnostic::error(message).with_location(usage.source_location().clone()),
            ));
        }

        let param_usages = mutation
            .param_usages()
            .iter()
            .map(|usage| {
                core::DbParamUsage::new(
                    usage.id().to_owned(),
                    self.param_type_for_mutation_usage(mutation, usage)
                        .unwrap_or_else(|| {
                            usage
                                .value_type_override()
                                .unwrap_or(core::CoreType::String)
                        }),
                )
            })
            .collect();

        Ok(core::DbMutationMetadata::new().with_param_usages(param_usages))
    }
}

impl FakeMetadataProvider {
    fn param_type_for_usage(
        &self,
        query: &core::RawQuery,
        usage: &core::ParamUsage,
    ) -> Option<core::CoreType> {
        let placeholder_index = usage.placeholder_index()?;
        let before_placeholder = &query.analysis_sql()[..placeholder_index];

        self.param_types_by_placeholder_prefix
            .iter()
            .find_map(|(prefix, ty)| before_placeholder.ends_with(prefix).then_some(*ty))
    }

    fn param_type_for_mutation_usage(
        &self,
        mutation: &core::RawMutation,
        usage: &core::ParamUsage,
    ) -> Option<core::CoreType> {
        let placeholder_index = usage.placeholder_index()?;
        let before_placeholder = &mutation.analysis_sql()[..placeholder_index];

        self.param_types_by_placeholder_prefix
            .iter()
            .find_map(|(prefix, ty)| before_placeholder.ends_with(prefix).then_some(*ty))
    }
}

#[derive(Clone, Debug)]
pub(super) struct LoggingQueryCompiler {
    calls: CallLog,
}

impl LoggingQueryCompiler {
    pub(super) const fn new(calls: CallLog) -> Self {
        Self { calls }
    }
}

impl QueryCompiler for LoggingQueryCompiler {
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        self.calls.push("compile");

        DefaultQueryCompiler.compile(query, analysis, metadata)
    }
}

impl MutationCompiler for LoggingQueryCompiler {
    fn compile_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
        metadata: &core::DbMutationMetadata,
    ) -> core::DiagnosticResult<core::CompiledMutation> {
        self.calls.push("compile_mutation");

        DefaultQueryCompiler.compile_mutation(mutation, analysis, metadata)
    }
}

#[derive(Clone, Debug)]
pub(super) struct FakeTargetGenerator {
    calls: CallLog,
    files: core::GeneratedFiles,
    failure: Option<PipelineFailure>,
    builders: Rc<RefCell<Vec<core::CompiledBuilder>>>,
}

impl FakeTargetGenerator {
    pub(super) fn new(calls: CallLog, files: core::GeneratedFiles) -> Self {
        Self {
            calls,
            files,
            failure: None,
            builders: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(super) const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Generation) {
            self.failure = Some(failure);
        }

        self
    }

    pub(super) fn generated_queries(&self) -> Vec<core::CompiledQuery> {
        self.builders
            .borrow()
            .iter()
            .filter_map(|builder| match builder {
                core::CompiledBuilder::Query(query) => Some(query.clone()),
                core::CompiledBuilder::Mutation(_) => None,
            })
            .collect()
    }

    pub(super) fn generated_builders(&self) -> Vec<core::CompiledBuilder> {
        self.builders.borrow().clone()
    }
}

impl TargetGenerator for FakeTargetGenerator {
    fn generate(
        &self,
        _plan: &core::CompilationPlan,
        builders: &[core::CompiledBuilder],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        self.calls.push("generate");

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        self.builders.borrow_mut().extend_from_slice(builders);

        Ok(self.files.clone())
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordingGeneratedFileWriter {
    calls: CallLog,
    files: Rc<RefCell<Vec<core::GeneratedFile>>>,
    cleaned: Rc<RefCell<Option<(PathBuf, core::GeneratedFiles)>>>,
}

impl RecordingGeneratedFileWriter {
    pub(super) fn new(calls: CallLog) -> Self {
        Self {
            calls,
            files: Rc::new(RefCell::new(Vec::new())),
            cleaned: Rc::new(RefCell::new(None)),
        }
    }

    pub(super) fn written_files(&self) -> Vec<core::GeneratedFile> {
        self.files.borrow().clone()
    }

    pub(super) fn cleaned_files(&self) -> Option<(PathBuf, core::GeneratedFiles)> {
        self.cleaned.borrow().clone()
    }
}

impl GeneratedFileWriter for RecordingGeneratedFileWriter {
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
        self.calls.push("write");
        self.files.borrow_mut().extend_from_slice(files.files());

        Ok(())
    }
}

impl GeneratedFileCleaner for RecordingGeneratedFileWriter {
    fn clean_stale(
        &self,
        output_dir: &Path,
        current_files: &core::GeneratedFiles,
    ) -> core::DiagnosticResult<usize> {
        self.calls.push("clean_stale");
        *self.cleaned.borrow_mut() = Some((output_dir.to_path_buf(), current_files.clone()));

        Ok(0)
    }
}
