use super::super::support::*;
use super::super::*;

#[test]
fn check_keeps_query_direct_param_and_repeat_item_param_inference_independent() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(vec![query]).with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_param_type_before_placeholder("u.email = ", core::CoreType::String)
        .with_param_type_before_placeholder("u.id IN (", core::CoreType::Int64)
        .with_param_type_before_placeholder(",", core::CoreType::Int64);
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
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_param_type_before_placeholder("email = ", core::CoreType::String)
        .with_param_type_before_placeholder("u.id IN (", core::CoreType::Int64)
        .with_param_type_before_placeholder(",", core::CoreType::Int64);
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
