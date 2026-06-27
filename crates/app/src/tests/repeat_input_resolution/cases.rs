use super::super::support::*;
use super::super::*;
use crate::CheckOutcome;

#[test]
fn check_rejects_repeat_id_collision_with_query_direct_param_id() {
    let sql = "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND u.id IN (?);";
    let direct_placeholder = sql.find('?').expect("direct placeholder exists");
    let repeat_placeholder = sql.rfind('?').expect("Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(vec![test_param_usage("ids", direct_placeholder)])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![
            test_param_usage("id", repeat_placeholder).with_sample_sql("1".to_owned()),
        ]),
    ])
    .with_source_path("sql/users.sql");

    let report = check_single_query(query)
        .expect_err("Repeat IDs must not collide with query direct Param IDs");

    assert_eq!(
        diagnostic_messages(&report),
        "Repeat `ids` in query `findUsers` conflicts with query direct Param `ids`; query direct Param IDs, Slot IDs, and Repeat IDs share the generated input namespace"
    );
}

#[test]
fn check_rejects_repeat_id_collision_with_query_slot_id() {
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?) AND 1 = 1;";
    let repeat_placeholder = sql.find('?').expect("Repeat placeholder exists");
    let slot_index = sql.find(';').expect("Slot insertion point exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filters".to_owned(),
        vec!["activeOnly".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "filters".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", repeat_placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned());

    let report = check_source_read(
        SourceRead::from_queries(vec![query])
            .with_fragments(vec![fragment])
            .with_source_file_count(2),
    )
    .expect_err("Repeat IDs must not collide with query Slot IDs");

    assert_eq!(
        diagnostic_messages(&report),
        "Repeat `filters` in query `findUsers` conflicts with Slot `filters`; query direct Param IDs, Slot IDs, and Repeat IDs share the generated input namespace"
    );
}

#[test]
fn check_rejects_repeat_id_collision_with_fragment_direct_param_id() {
    let query_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = query_sql.find(';').expect("Slot insertion point exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        query_sql.to_owned(),
    )
    .with_analysis_sql(query_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byIds".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");

    let fragment_sql = "\nAND u.tenant_id = ? AND u.id IN (?)";
    let direct_placeholder = fragment_sql
        .find('?')
        .expect("fragment direct placeholder exists");
    let repeat_placeholder = fragment_sql
        .rfind('?')
        .expect("fragment Repeat placeholder exists");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byIds".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![test_param_usage("ids", direct_placeholder)])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", repeat_placeholder)]),
    ]);

    let report = check_source_read(
        SourceRead::from_queries(vec![query])
            .with_fragments(vec![fragment])
            .with_source_file_count(2),
    )
    .expect_err("Fragment Repeat IDs must not collide with Fragment direct Param IDs");

    assert_eq!(
        diagnostic_messages(&report),
        "Repeat `ids` in Fragment `byIds` selected by Slot `filter` in query `findUsers` conflicts with Fragment direct Param `ids`; Fragment direct Param IDs and Repeat IDs share the selected Slot branch input namespace"
    );
}

#[test]
fn check_rejects_repeated_query_repeat_id_with_incompatible_item_shape() {
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?) OR u.email IN (?);";
    let first_placeholder = sql.find('?').expect("first Repeat placeholder exists");
    let second_placeholder = sql.rfind('?').expect("second Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            first_placeholder,
            first_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(first_placeholder),
        ]),
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            second_placeholder,
            second_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "email".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(second_placeholder),
        ]),
    ])
    .with_source_path("sql/users.sql");

    let report = check_single_query(query)
        .expect_err("repeated Repeat IDs with different item Param ID sets should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Repeat `values` item shape in query `findUsers`: first occurrence uses fields [id] but conflicting occurrence uses [email]; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability"
    );
}

#[test]
fn check_rejects_repeated_query_repeat_id_with_item_value_type_conflict() {
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?) OR u.id IN (?);";
    let first_placeholder = sql.find('?').expect("first Repeat placeholder exists");
    let second_placeholder = sql.rfind('?').expect("second Repeat placeholder exists");
    let first_repeat_location = test_location(5, 14);
    let second_repeat_location = test_location(6, 14);
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            first_placeholder,
            first_placeholder + 1,
            first_repeat_location,
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(first_placeholder),
        ]),
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            second_placeholder,
            second_placeholder + 1,
            second_repeat_location.clone(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(second_placeholder),
        ]),
    ])
    .with_source_path("sql/users.sql");

    let report = check_single_query(query)
        .expect_err("repeated Repeat item Params with conflicting valueTypes should be rejected");

    assert_eq!(
        report.diagnostics()[0].location(),
        Some(&second_repeat_location)
    );
    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Repeat `values` item shape in query `findUsers` item Param `id` type conflict: first occurrence uses Int64 but conflicting occurrence uses String; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability"
    );
}

#[test]
fn check_rejects_repeated_query_repeat_id_with_item_nullability_conflict() {
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?) OR u.id IN (?);";
    let first_placeholder = sql.find('?').expect("first Repeat placeholder exists");
    let second_placeholder = sql.rfind('?').expect("second Repeat placeholder exists");
    let first_repeat_location = test_location(8, 14);
    let second_repeat_location = test_location(9, 14);
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            first_placeholder,
            first_placeholder + 1,
            first_repeat_location,
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(first_placeholder),
        ]),
        core::RepeatUsage::new(
            "values".to_owned(),
            ",".to_owned(),
            second_placeholder,
            second_placeholder + 1,
            second_repeat_location.clone(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                true,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(second_placeholder),
        ]),
    ])
    .with_source_path("sql/users.sql");

    let report = check_single_query(query)
        .expect_err("repeated Repeat item Params with conflicting nullability should be rejected");

    assert_eq!(
        report.diagnostics()[0].location(),
        Some(&second_repeat_location)
    );
    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Repeat `values` item shape in query `findUsers` item Param `id` nullability conflict: first occurrence is nullable false but conflicting occurrence is nullable true; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability"
    );
}

#[test]
fn check_allows_repeated_query_repeat_id_with_different_item_param_order() {
    let sql = "SELECT u.id FROM users AS u WHERE (u.id, u.kind) IN ((?, ?)) OR (u.kind, u.id) IN ((?, ?));";
    let placeholders = sql
        .match_indices('?')
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "pairs".to_owned(),
            ",".to_owned(),
            placeholders[0],
            placeholders[1] + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(placeholders[0]),
            core::ParamUsage::new(
                "kind".to_owned(),
                Some(core::CoreType::String),
                true,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(placeholders[1]),
        ]),
        core::RepeatUsage::new(
            "pairs".to_owned(),
            "|".to_owned(),
            placeholders[2],
            placeholders[3] + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![
            core::ParamUsage::new(
                "kind".to_owned(),
                Some(core::CoreType::String),
                true,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(placeholders[2]),
            core::ParamUsage::new(
                "id".to_owned(),
                Some(core::CoreType::Int64),
                false,
                core::SourceLocation::unknown(),
            )
            .with_placeholder_index(placeholders[3]),
        ]),
    ])
    .with_source_path("sql/users.sql");

    let outcome = check_single_query(query)
        .expect("compatible repeated Repeat IDs should share one array input");

    assert_eq!(outcome.query_summaries()[0].id(), "findUsers");
}

#[test]
fn check_rejects_repeat_id_collision_with_mutation_direct_param_id() {
    let sql = "INSERT INTO users (tenant_id, id) VALUES (?, ?);";
    let direct_placeholder = sql.find('?').expect("direct placeholder exists");
    let repeat_placeholder = sql.rfind('?').expect("Repeat placeholder exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_param_usages(vec![test_param_usage("rows", direct_placeholder)])
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "rows".to_owned(),
            ",".to_owned(),
            repeat_placeholder,
            repeat_placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", repeat_placeholder)]),
    ])
    .with_source_path("sql/users.sql");

    let report = check_source_read(
        SourceRead::from_queries(Vec::new())
            .with_mutations(vec![mutation])
            .with_source_file_count(1),
    )
    .expect_err("Repeat IDs must not collide with mutation direct Param IDs");

    assert_eq!(
        diagnostic_messages(&report),
        "Repeat `rows` in mutation `createUsers` conflicts with mutation direct Param `rows`; mutation direct Param IDs, Slot IDs, and Repeat IDs share the generated input namespace"
    );
}

fn check_single_query(query: core::RawQuery) -> core::DiagnosticResult<CheckOutcome> {
    check_source_read(SourceRead::from_queries(vec![query]).with_source_file_count(1))
}

fn check_source_read(source_read: SourceRead) -> core::DiagnosticResult<CheckOutcome> {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
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

    DefaultCompileUseCase::check(&config, &pipeline)
}

fn test_location(line: usize, column: usize) -> core::SourceLocation {
    core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(line, column).expect("test position should be valid"),
    )
}
