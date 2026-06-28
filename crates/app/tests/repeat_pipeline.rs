use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use sqlay_app::{
    CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase, GeneratedFileWriter,
    MetadataProvider, MutationCompiler, MutationMetadataProvider, QueryCompiler, SourceRead,
    SourceReader, TargetGenerator,
};
use sqlay_core as core;

#[test]
fn check_keeps_query_direct_param_and_repeat_item_param_inference_independent() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let sql = "SELECT u.id FROM users AS u WHERE u.email = ? AND u.id IN (?);";
    let direct_placeholder = sql.find('?').expect("direct placeholder exists");
    let repeat_placeholder = sql.rfind('?').expect("Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(vec![test_param_usage("id", direct_placeholder)])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", repeat_placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let source_reader =
        FakeSourceReader::new(SourceRead::from_queries(vec![query]).with_source_file_count(1));
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new()
        .with_param_type_before_placeholder("u.email = ", core::CoreType::String)
        .with_param_type_before_placeholder("u.id IN (", core::CoreType::Int64)
        .with_param_type_before_placeholder(",", core::CoreType::Int64);
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
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
        .expect("query direct Param and Repeat item Param namespaces should be independent");

    let generated_queries = target_generator.generated_queries();
    assert_eq!(generated_queries.len(), 1);
    assert_eq!(
        generated_queries[0].input(),
        [core::InputField::new(
            "id".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(
        generated_queries[0].params(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
}

#[test]
fn check_keeps_mutation_direct_param_and_repeat_item_param_inference_independent() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let sql = "UPDATE users AS u SET email = ? WHERE u.id IN (?);";
    let direct_placeholder = sql.find('?').expect("direct placeholder exists");
    let repeat_placeholder = sql.rfind('?').expect("Repeat placeholder exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("updateUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(vec![test_param_usage("id", direct_placeholder)])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", repeat_placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let source_reader = FakeSourceReader::new(
        SourceRead::from_queries(Vec::new())
            .with_mutations(vec![mutation.clone()])
            .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
            .with_source_file_count(1),
    );
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new()
        .with_param_type_before_placeholder("email = ", core::CoreType::String)
        .with_param_type_before_placeholder("u.id IN (", core::CoreType::Int64)
        .with_param_type_before_placeholder(",", core::CoreType::Int64);
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
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
        .expect("mutation direct Param and Repeat item Param namespaces should be independent");

    let generated_builders = target_generator.generated_builders();
    assert_eq!(generated_builders.len(), 1);
    let core::CompiledBuilder::Mutation(compiled) = &generated_builders[0] else {
        panic!("target generator should receive a mutation builder");
    };
    assert_eq!(
        compiled.input(),
        [core::InputField::new(
            "id".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(
        compiled.params(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
}

#[test]
fn check_expands_query_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            placeholder,
            placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let source_reader =
        FakeSourceReader::new(SourceRead::from_queries(vec![query]).with_source_file_count(1));
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new();
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("query Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["SELECT u.id FROM users AS u WHERE u.id IN (?,?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["id".to_owned(), "id".to_owned()]]
    );

    let generated_queries = target_generator.generated_queries();
    assert_eq!(generated_queries.len(), 1);
    let dynamic = generated_queries[0]
        .dynamic_body()
        .expect("query Repeat should carry dynamic Core IR");
    assert_eq!(dynamic.repeats().len(), 1);
    assert_eq!(dynamic.repeats()[0].id(), "ids");
    assert_eq!(
        dynamic.repeats()[0].fields(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false
        )]
    );
    assert_eq!(dynamic.base_bodies().len(), 1);
    let body = &dynamic.base_bodies()[0];
    assert_eq!(body.base_segments().len(), 2);
    assert_eq!(
        body.base_segments()[0].sql(),
        "SELECT u.id FROM users AS u WHERE u.id IN ("
    );
    assert!(body.base_segments()[0].params().is_empty());
    assert_eq!(body.base_segments()[1].sql(), ");");
    assert_eq!(body.repeat_occurrences().len(), 1);
    assert_eq!(body.repeat_occurrences()[0].repeat_id(), "ids");
    assert_eq!(body.repeat_occurrences()[0].separator(), ",");
    assert_eq!(body.repeat_occurrences()[0].item_segment().sql(), "?");
    assert_eq!(
        body.repeat_occurrences()[0].item_segment().params(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false
        )]
    );
}

#[test]
fn check_combines_slot_variants_with_fragment_repeat_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let query_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = query_sql.find(';').expect("Slot insertion point exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [byIds] } */;"
            .to_owned(),
    )
    .with_analysis_sql(query_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byIds".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = "\nAND u.id IN (?)";
    let placeholder = fragment_sql.find('?').expect("Repeat placeholder exists");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byIds".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            placeholder,
            placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", placeholder)]),
    ])
    .with_source_path("sql/fragments.sql");
    let source_reader = FakeSourceReader::new(
        SourceRead::from_queries(vec![query.clone()])
            .with_fragments(vec![fragment])
            .with_source_units(vec![core::RawSourceUnit::Query(query)])
            .with_source_file_count(2),
    );
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new();
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("Slot-selected Fragment Repeat should validate with representative SQL");

    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.id IN (?,?);",
        ]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [Vec::<String>::new(), vec!["id".to_owned(), "id".to_owned()]]
    );

    let generated_queries = target_generator.generated_queries();
    assert_eq!(generated_queries.len(), 1);
    let dynamic = generated_queries[0]
        .dynamic_body()
        .expect("Slot-selected Fragment Repeat should carry dynamic Core IR");
    assert_slot_fragment_repeat_core_ir(dynamic);
}

#[test]
fn check_expands_mutation_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let sql = "INSERT INTO users (email) VALUES (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "rows".to_owned(),
            ",".to_owned(),
            placeholder - 1,
            placeholder + 2,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("email", placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let source_reader = FakeSourceReader::new(
        SourceRead::from_queries(Vec::new())
            .with_mutations(vec![mutation.clone()])
            .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
            .with_source_file_count(1),
    );
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new();
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("mutation Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["INSERT INTO users (email) VALUES (?),(?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["email".to_owned(), "email".to_owned()]]
    );

    let generated_builders = target_generator.generated_builders();
    assert_eq!(generated_builders.len(), 1);
    let core::CompiledBuilder::Mutation(compiled) = &generated_builders[0] else {
        panic!("target generator should receive a mutation builder");
    };
    let dynamic = compiled
        .dynamic_body()
        .expect("mutation Repeat should carry dynamic Core IR");
    assert_eq!(dynamic.repeats().len(), 1);
    assert_eq!(dynamic.repeats()[0].id(), "rows");
    assert_eq!(
        dynamic.repeats()[0].fields(),
        [core::ParamBinding::new(
            "email".to_owned(),
            core::CoreType::String,
            false
        )]
    );
    assert_eq!(dynamic.base_bodies().len(), 1);
    let body = &dynamic.base_bodies()[0];
    assert_eq!(body.base_segments().len(), 2);
    assert_eq!(
        body.base_segments()[0].sql(),
        "INSERT INTO users (email) VALUES "
    );
    assert_eq!(body.base_segments()[1].sql(), ";");
    assert_eq!(body.repeat_occurrences().len(), 1);
    assert_eq!(body.repeat_occurrences()[0].repeat_id(), "rows");
    assert_eq!(body.repeat_occurrences()[0].separator(), ",");
    assert_eq!(body.repeat_occurrences()[0].item_segment().sql(), "(?)");
    assert_eq!(
        body.repeat_occurrences()[0].item_segment().params(),
        [core::ParamBinding::new(
            "email".to_owned(),
            core::CoreType::String,
            false
        )]
    );
}

#[test]
fn check_combines_mutation_slot_variants_with_fragment_repeat_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let mutation_sql = "UPDATE users AS u SET name = name WHERE 1 = 1;";
    let slot_index = mutation_sql.find(';').expect("Slot insertion point exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("touchUsers".to_owned()),
        "UPDATE users AS u SET name = name WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [byIds] } */;"
            .to_owned(),
    )
    .with_analysis_sql(mutation_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byIds".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = "\nAND u.id IN (?)";
    let placeholder = fragment_sql.find('?').expect("Repeat placeholder exists");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byIds".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            placeholder,
            placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", placeholder)]),
    ])
    .with_source_path("sql/mutation_fragments.sql");
    let source_reader = FakeSourceReader::new(
        SourceRead::from_queries(Vec::new())
            .with_mutations(vec![mutation.clone()])
            .with_fragments(vec![fragment])
            .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
            .with_source_file_count(2),
    );
    let dialect_analyzer = FakeDialectAnalyzer::default();
    let metadata_provider = FakeMetadataProvider::new();
    let query_compiler = DefaultingQueryCompiler;
    let target_generator = RecordingTargetGenerator::default();
    let generated_file_writer = NoopGeneratedFileWriter;
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("Slot-selected mutation Fragment Repeat should validate representative SQL");

    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "UPDATE users AS u SET name = name WHERE 1 = 1;",
            "UPDATE users AS u SET name = name WHERE 1 = 1\nAND u.id IN (?,?);",
        ]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [Vec::<String>::new(), vec!["id".to_owned(), "id".to_owned()]]
    );

    let generated_builders = target_generator.generated_builders();
    assert_eq!(generated_builders.len(), 1);
    let core::CompiledBuilder::Mutation(compiled) = &generated_builders[0] else {
        panic!("target generator should receive a mutation builder");
    };
    let dynamic = compiled
        .dynamic_body()
        .expect("Slot-selected mutation Fragment Repeat should carry dynamic Core IR");
    assert_slot_fragment_repeat_core_ir(dynamic);
}

fn assert_slot_fragment_repeat_core_ir(dynamic: &core::CompiledDynamicQuery) {
    assert!(dynamic.repeats().is_empty());
    assert_eq!(dynamic.slots().len(), 1);
    let branch = &dynamic.slots()[0].branches()[0];
    assert_eq!(branch.repeats().len(), 1);
    assert_eq!(branch.repeats()[0].id(), "ids");
    assert_eq!(
        branch.repeats()[0].fields(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false
        )]
    );
    let body = branch.body();
    assert_eq!(body.base_segments().len(), 2);
    assert_eq!(body.base_segments()[0].sql(), "\nAND u.id IN (");
    assert_eq!(body.base_segments()[1].sql(), ")");
    assert_eq!(body.repeat_occurrences().len(), 1);
    assert_eq!(body.repeat_occurrences()[0].repeat_id(), "ids");
    assert_eq!(body.repeat_occurrences()[0].separator(), ",");
    assert_eq!(body.repeat_occurrences()[0].item_segment().sql(), "?");
    assert_eq!(
        body.repeat_occurrences()[0].item_segment().params(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false
        )]
    );
}

fn project_config(config_dir: PathBuf) -> core::ProjectConfig {
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

fn test_param_usage(id: &str, placeholder_index: usize) -> core::ParamUsage {
    core::ParamUsage::new(id.to_owned(), None, false, core::SourceLocation::unknown())
        .with_placeholder_index(placeholder_index)
}

#[derive(Clone, Debug)]
struct FakeSourceReader {
    source_read: SourceRead,
}

impl FakeSourceReader {
    const fn new(source_read: SourceRead) -> Self {
        Self { source_read }
    }
}

impl SourceReader for FakeSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        Ok(self.source_read.clone())
    }
}

#[derive(Clone, Debug, Default)]
struct FakeDialectAnalyzer {
    analyzed_sql: Rc<RefCell<Vec<String>>>,
}

impl FakeDialectAnalyzer {
    fn analyzed_sql(&self) -> Vec<String> {
        self.analyzed_sql.borrow().clone()
    }
}

impl sqlay_app::DialectAnalyzer for FakeDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        self.analyzed_sql
            .borrow_mut()
            .push(query.analysis_sql().to_owned());
        Ok(core::AnalyzedQuery::new(core::Cardinality::Many))
    }
}

impl sqlay_app::MutationAnalyzer for FakeDialectAnalyzer {
    fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation> {
        self.analyzed_sql
            .borrow_mut()
            .push(mutation.analysis_sql().to_owned());
        let kind = if mutation.analysis_sql().trim_start().starts_with("INSERT") {
            core::MutationKind::Insert
        } else {
            core::MutationKind::Update
        };

        Ok(core::AnalyzedMutation::new(kind))
    }
}

#[derive(Clone, Debug, Default)]
struct FakeMetadataProvider {
    param_types_by_placeholder_prefix: Vec<(&'static str, core::CoreType)>,
    described_param_ids: Rc<RefCell<Vec<Vec<String>>>>,
}

impl FakeMetadataProvider {
    fn new() -> Self {
        Self::default()
    }

    fn with_param_type_before_placeholder(
        mut self,
        sql_prefix: &'static str,
        ty: core::CoreType,
    ) -> Self {
        self.param_types_by_placeholder_prefix
            .push((sql_prefix, ty));
        self
    }

    fn described_param_ids(&self) -> Vec<Vec<String>> {
        self.described_param_ids.borrow().clone()
    }

    fn param_type_for_usage(
        &self,
        analysis_sql: &str,
        usage: &core::ParamUsage,
    ) -> Option<core::CoreType> {
        let placeholder_index = usage.placeholder_index()?;
        let before_placeholder = &analysis_sql[..placeholder_index];

        self.param_types_by_placeholder_prefix
            .iter()
            .find_map(|(prefix, ty)| before_placeholder.ends_with(prefix).then_some(*ty))
    }
}

impl MetadataProvider for FakeMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        self.described_param_ids.borrow_mut().push(
            query
                .param_usages()
                .iter()
                .map(|usage| usage.id().to_owned())
                .collect(),
        );
        let param_usages = query
            .param_usages()
            .iter()
            .map(|usage| {
                core::DbParamUsage::new(
                    usage.id().to_owned(),
                    self.param_type_for_usage(query.analysis_sql(), usage)
                        .unwrap_or(core::CoreType::String),
                )
            })
            .collect();

        Ok(core::DbQueryMetadata::new(vec![core::DbResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            Some(false),
        )])
        .with_param_usages(param_usages))
    }
}

impl MutationMetadataProvider for FakeMetadataProvider {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        _analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        self.described_param_ids.borrow_mut().push(
            mutation
                .param_usages()
                .iter()
                .map(|usage| usage.id().to_owned())
                .collect(),
        );
        let param_usages = mutation
            .param_usages()
            .iter()
            .map(|usage| {
                core::DbParamUsage::new(
                    usage.id().to_owned(),
                    self.param_type_for_usage(mutation.analysis_sql(), usage)
                        .unwrap_or(core::CoreType::String),
                )
            })
            .collect();

        Ok(core::DbMutationMetadata::new().with_param_usages(param_usages))
    }
}

#[derive(Clone, Copy, Debug)]
struct DefaultingQueryCompiler;

impl QueryCompiler for DefaultingQueryCompiler {
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        sqlay_app::QueryCompiler::compile(
            &sqlay_app::DefaultQueryCompiler,
            query,
            analysis,
            metadata,
        )
    }
}

impl MutationCompiler for DefaultingQueryCompiler {
    fn compile_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
        metadata: &core::DbMutationMetadata,
    ) -> core::DiagnosticResult<core::CompiledMutation> {
        sqlay_app::MutationCompiler::compile_mutation(
            &sqlay_app::DefaultQueryCompiler,
            mutation,
            analysis,
            metadata,
        )
    }
}

#[derive(Clone, Debug, Default)]
struct RecordingTargetGenerator {
    builders: Rc<RefCell<Vec<core::CompiledBuilder>>>,
}

impl RecordingTargetGenerator {
    fn generated_queries(&self) -> Vec<core::CompiledQuery> {
        self.builders
            .borrow()
            .iter()
            .filter_map(|builder| match builder {
                core::CompiledBuilder::Query(query) => Some(query.clone()),
                core::CompiledBuilder::Mutation(_) => None,
            })
            .collect()
    }

    fn generated_builders(&self) -> Vec<core::CompiledBuilder> {
        self.builders.borrow().clone()
    }
}

impl TargetGenerator for RecordingTargetGenerator {
    fn generate(
        &self,
        _plan: &core::CompilationPlan,
        builders: &[core::CompiledBuilder],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        self.builders.borrow_mut().extend_from_slice(builders);

        Ok(core::GeneratedFiles::new(Vec::new()))
    }
}

#[derive(Clone, Copy, Debug)]
struct NoopGeneratedFileWriter;

impl GeneratedFileWriter for NoopGeneratedFileWriter {
    fn write(&self, _files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
        Ok(())
    }
}
