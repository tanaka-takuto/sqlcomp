use super::super::support::*;
use super::super::*;

#[test]
fn check_reports_unknown_mutation_slot_target_at_mutation_location_when_slot_location_is_unknown() {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [missingAssignment] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql("UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["missingAssignment".to_owned()],
        "UPDATE users AS u SET name = ?".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());

    let (report, calls) = check_mutation_only_source_error(mutation, Vec::new(), 1);

    assert_eq!(
        diagnostic_messages(&report),
        "unknown Slot target `missingAssignment` in Slot `assignment`; no fragment with that id was found"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_mutation_slot_param_collision_at_mutation_location_when_slot_location_is_unknown()
{
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: name targets: [touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql("UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "name".to_owned(),
        vec!["touchUpdatedAt".to_owned()],
        "UPDATE users AS u SET name = ?".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());
    let fragment = mutation_fragment_without_params("touchUpdatedAt", ", updated_at = NOW()");

    let (report, calls) = check_mutation_only_source_error(mutation, vec![fragment], 2);

    assert_eq!(
        diagnostic_messages(&report),
        "Slot `name` in mutation `renameUser` conflicts with mutation direct Param `name`; mutation direct Param IDs and Slot IDs share the generated input namespace"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_duplicate_mutation_slot_targets_at_mutation_location_when_slot_location_is_unknown()
 {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [touchUpdatedAt, touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql("UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["touchUpdatedAt".to_owned(), "touchUpdatedAt".to_owned()],
        "UPDATE users AS u SET name = ?".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());

    let (report, calls) = check_mutation_only_source_error(mutation, Vec::new(), 1);

    assert_eq!(
        diagnostic_messages(&report),
        "duplicate Slot target `touchUpdatedAt` in Slot `assignment`; each target must appear at most once in `targets`"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_conflicting_mutation_slot_targets_at_mutation_location_when_slot_location_is_unknown()
 {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [touchUpdatedAt, auditName] } *//* @sqlay { type: slot id: assignment targets: [auditName, touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql("UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "assignment".to_owned(),
            vec!["touchUpdatedAt".to_owned(), "auditName".to_owned()],
            "UPDATE users AS u SET name = ?".len(),
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "assignment".to_owned(),
            vec!["auditName".to_owned(), "touchUpdatedAt".to_owned()],
            "UPDATE users AS u SET name = ?".len(),
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());

    let (report, calls) = check_mutation_only_source_error(mutation, Vec::new(), 1);

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Slot `assignment` targets in mutation `renameUser`: first occurrence uses [touchUpdatedAt, auditName] but conflicting occurrence uses [auditName, touchUpdatedAt]; repeated Slot IDs must use the same `targets` values in the same order"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_mutation_slot_variant_limit_at_mutation_location() {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let mut slot_usages = Vec::new();
    for index in 0..5 {
        slot_usages.push(core::SlotUsage::new(
            format!("assignment{index}"),
            vec![
                "touchUpdatedAt".to_owned(),
                "auditName".to_owned(),
                "touchDeletedAt".to_owned(),
            ],
            0,
            core::SourceLocation::unknown(),
        ));
    }
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned(),
    )
    .with_analysis_sql("UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(slot_usages)
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());
    let fragments = vec![
        mutation_fragment_without_params("touchUpdatedAt", ", updated_at = NOW()"),
        mutation_fragment_without_params("auditName", ", audit_name = name"),
        mutation_fragment_without_params("touchDeletedAt", ", deleted_at = NULL"),
    ];

    let (report, calls) = check_mutation_only_source_error(mutation, fragments, 4);

    assert_eq!(
        diagnostic_messages(&report),
        "Dynamic SQL validation for mutation `renameUser` would produce 1024 validation cases, exceeding the 256 validation case limit"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_invalid_mutation_slot_insertion_index_at_mutation_location_when_slot_location_is_unknown()
 {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let analysis_sql = "UPDATE users AS u SET name = ? WHERE u.id = ?;";
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned(),
    )
    .with_analysis_sql(analysis_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["touchUpdatedAt".to_owned()],
        analysis_sql.len() + 1,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());
    let fragments = vec![mutation_fragment_without_params(
        "touchUpdatedAt",
        ", updated_at = NOW()",
    )];

    let (report, calls) = check_mutation_only_source_error(mutation, fragments, 2);

    assert_eq!(
        diagnostic_messages(&report),
        "invalid Slot `assignment` insertion index 47 for mutation analysis SQL"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_out_of_order_mutation_param_usage_at_mutation_location() {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let analysis_sql = "UPDATE users AS u SET name = ? WHERE u.id = ?;";
    let first_placeholder = analysis_sql.find('?').expect("first placeholder exists");
    let second_placeholder = analysis_sql.rfind('?').expect("second placeholder exists");
    let slot_index = "UPDATE users AS u SET name = ?".len();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql(analysis_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage("name", first_placeholder),
        test_param_usage("id", second_placeholder),
        test_param_usage("nameAgain", first_placeholder),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["touchUpdatedAt".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());
    let fragments = vec![mutation_fragment_without_params(
        "touchUpdatedAt",
        ", updated_at = NOW()",
    )];

    let (report, calls) = check_mutation_only_source_error(mutation, fragments, 2);

    assert_eq!(
        diagnostic_messages(&report),
        format!(
            "Param `nameAgain` placeholder index {first_placeholder} appears before the current Slot expansion cursor {slot_index}"
        )
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

#[test]
fn check_reports_mutation_fragment_param_without_placeholder_at_mutation_location() {
    let mutation_location = core::SourceLocation::for_path("sql/users.sql");
    let analysis_sql = "UPDATE users AS u SET name = ? WHERE u.id = ?;";
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql(analysis_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage("name", "UPDATE users AS u SET name = ".len()),
        test_param_usage("id", "UPDATE users AS u SET name = ? WHERE u.id = ".len()),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["touchUpdatedAt".to_owned()],
        "UPDATE users AS u SET name = ?".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql")
    .with_source_location(mutation_location.clone());
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("touchUpdatedAt".to_owned()),
        ", updated_at = ?".to_owned(),
    )
    .with_analysis_sql(", updated_at = ?".to_owned())
    .with_param_usages(vec![core::ParamUsage::new(
        "updatedAt".to_owned(),
        None,
        false,
        core::SourceLocation::unknown(),
    )]);

    let (report, calls) = check_mutation_only_source_error(mutation, vec![fragment], 2);

    assert_eq!(
        diagnostic_messages(&report),
        "Param `updatedAt` in fragment `touchUpdatedAt` is missing placeholder position metadata"
    );
    assert_eq!(report.diagnostics()[0].location(), Some(&mutation_location));
    assert_eq!(calls, ["read"]);
}

fn mutation_fragment_without_params(id: &str, sql: &str) -> core::RawFragment {
    core::RawFragment::new(core::FragmentMetadata::new(id.to_owned()), sql.to_owned())
        .with_analysis_sql(sql.to_owned())
}

fn check_mutation_only_source_error(
    mutation: core::RawMutation,
    fragments: Vec<core::RawFragment>,
    source_file_count: usize,
) -> (core::DiagnosticReport, Vec<&'static str>) {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_fragments(fragments)
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(source_file_count);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator =
        FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()));
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls.clone());
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    (
        DefaultCompileUseCase::check(&config, &pipeline)
            .expect_err("mutation Slot fixture should fail before generation"),
        calls.entries(),
    )
}
