use super::super::support::*;
use super::super::*;

#[test]
fn check_expands_query_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(vec![query]).with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("query Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(outcome.validation_case_count(), 1);
    assert_eq!(outcome.unique_repeat_count(), 1);
    assert_eq!(outcome.query_summaries()[0].repeat_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["SELECT u.id FROM users AS u WHERE u.id IN (?,?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["id".to_owned(), "id".to_owned()]]
    );
}

#[test]
fn check_combines_slot_variants_with_fragment_repeat_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(vec![query.clone()])
        .with_fragments(vec![fragment])
        .with_source_units(vec![core::RawSourceUnit::Query(query)])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("Slot-selected Fragment Repeat should validate with representative SQL");

    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(outcome.validation_case_count(), 2);
    assert_eq!(outcome.unique_repeat_count(), 1);
    assert_eq!(outcome.query_summaries()[0].repeat_count(), 1);
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
}

#[test]
fn check_expands_mutation_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("mutation Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(outcome.validation_case_count(), 1);
    assert_eq!(outcome.unique_repeat_count(), 1);
    assert_eq!(outcome.mutation_summaries()[0].repeat_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["INSERT INTO users (email) VALUES (?),(?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["email".to_owned(), "email".to_owned()]]
    );
}

#[test]
fn check_combines_mutation_slot_variants_with_fragment_repeat_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_fragments(vec![fragment])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("Slot-selected mutation Fragment Repeat should validate representative SQL");

    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(outcome.validation_case_count(), 2);
    assert_eq!(outcome.unique_repeat_count(), 1);
    assert_eq!(outcome.mutation_summaries()[0].repeat_count(), 1);
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
}
