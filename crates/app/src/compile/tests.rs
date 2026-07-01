use super::diagnostics::with_slot_variant_context;
use super::dynamic_ir::{compile_dynamic_mutation_body, compile_dynamic_query_body};
use super::param_validation::{
    ScopedParamBinding, validate_expanded_mutation_variant_param_bindings,
    validate_expanded_variant_param_bindings,
};
use super::slot_variants::{
    AnalyzedMutationVariant, AnalyzedQueryVariant, ExpandedFragmentParamOccurrence,
    ExpandedFragmentRepeatParamOccurrence, ExpandedParamOccurrence, ExpandedParamScope,
    ExpandedRepeatParamOccurrence, SlotExpansionContext, SlotExpansionSourceKind,
    SlotSelectionContext,
};
use super::*;
use std::collections::HashMap;
use std::path::Path;

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

    let summary = QuerySummary::from_compiled_query(&query, 2, 3, 6);

    assert_eq!(summary.id(), "filterUsers");
    assert_eq!(summary.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(summary.param_count(), 3);
    assert_eq!(summary.input_field_count(), 2);
    assert_eq!(summary.slot_count(), 2);
    assert_eq!(summary.repeat_count(), 3);
    assert_eq!(summary.variant_count(), 6);
    assert_eq!(summary.validation_case_count(), 6);
}

#[test]
fn mutation_summary_counts_param_placeholders_and_input_fields_separately() {
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("createUser".to_owned()),
        "INSERT INTO users (email, name) VALUES (?, ?);".to_owned(),
        core::MutationKind::Insert,
        vec![
            core::InputField::new("email".to_owned(), core::CoreType::String, false),
            core::InputField::new("name".to_owned(), core::CoreType::String, true),
        ],
    )
    .with_source_path("sql/users.sql")
    .with_params(vec![
        core::ParamBinding::new("email".to_owned(), core::CoreType::String, false),
        core::ParamBinding::new("name".to_owned(), core::CoreType::String, true),
    ]);

    let summary = MutationSummary::from_compiled_mutation(&mutation, 0, 1, 1);

    assert_eq!(summary.id(), "createUser");
    assert_eq!(summary.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(summary.kind(), core::MutationKind::Insert);
    assert_eq!(summary.param_count(), 2);
    assert_eq!(summary.input_field_count(), 2);
    assert_eq!(summary.slot_count(), 0);
    assert_eq!(summary.repeat_count(), 1);
    assert_eq!(summary.variant_count(), 1);
    assert_eq!(summary.validation_case_count(), 1);
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
    let expected_first_slot_location = first_slot_location.clone();
    let expected_second_slot_location = second_slot_location.clone();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users WHERE first = ? OR second = ?;".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("kind", 35, false),
        test_param_usage("kind", 49, true),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: Some(SlotExpansionContext {
            source_kind: SlotExpansionSourceKind::Query,
            source_id: "listUsers".to_owned(),
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

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))
            .expect_err("repeated Slot Fragment Param nullability conflicts should be rejected");

    assert_diagnostic_messages(
        &report,
        "conflicting Fragment Param `kind` nullability in query `listUsers`, Slot `filter`, Fragment `byKind`: occurrence 1 is nullable false but occurrence 2 is nullable true; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability\nfirst occurrence of Slot `filter` selecting Fragment `byKind` is here\nconflicting occurrence of Slot `filter` selecting Fragment `byKind` is here\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=byKind\nSlot `filter` selected `byKind` in this variant",
    );
    assert_eq!(
        report.diagnostics()[1].location(),
        Some(&expected_first_slot_location)
    );
    assert_eq!(
        report.diagnostics()[2].location(),
        Some(&expected_second_slot_location)
    );
    assert_eq!(
        report.diagnostics()[4].location(),
        Some(&expected_first_slot_location)
    );
}

#[test]
fn fragment_repeat_param_validation_reports_slot_fragment_and_repeat_context() {
    let slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(8, 88).expect("test position should be valid"),
    );
    let repeat_location = core::SourceLocation::at_position(
        "sql/fragments.sql",
        core::SourcePosition::one_based(3, 14).expect("test position should be valid"),
    );
    let expected_slot_location = slot_location.clone();
    let expected_repeat_location = repeat_location.clone();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users WHERE id IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("id", 34, false),
        test_param_usage("id", 36, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: Some(SlotExpansionContext {
            source_kind: SlotExpansionSourceKind::Query,
            source_id: "listUsers".to_owned(),
            selections: vec![SlotSelectionContext {
                slot_id: "filter".to_owned(),
                target_id: Some("byIds".to_owned()),
                slot_location: slot_location.clone(),
                fragment_location: None,
            }],
        }),
        param_scopes: vec![
            ExpandedParamScope::FragmentRepeatItem {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
            },
            ExpandedParamScope::FragmentRepeatItem {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
            },
        ],
        param_occurrences: vec![
            ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
                representative_item_index: 1,
                slot_occurrence_index: 1,
                slot_location,
                repeat_location: repeat_location.clone(),
            }),
            ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
                representative_item_index: 2,
                slot_occurrence_index: 1,
                slot_location: core::SourceLocation::unknown(),
                repeat_location,
            }),
        ],
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("id".to_owned(), core::CoreType::String),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))
            .expect_err("Fragment Repeat Param type conflicts should include context");

    assert_diagnostic_messages(
        &report,
        "conflicting Fragment Repeat item Param `id` type in query `listUsers`, Slot `filter`, Fragment `byIds`, Repeat `ids`: first representative occurrence resolved to Int64 but conflicting representative occurrence resolved to String; Repeat item fields with the same ID must resolve matching Param type and nullability\nfirst Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nconflicting Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=byIds\nSlot `filter` selected `byIds` in this variant",
    );
    assert_eq!(
        report.diagnostics()[1].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[2].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[4].location(),
        Some(&expected_slot_location)
    );
}

#[test]
fn query_repeat_param_validation_reports_repeat_locations() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(4, 12).expect("test position should be valid"),
    );
    let expected_repeat_location = repeat_location.clone();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        "SELECT id FROM users WHERE id IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("id", 34, false),
        test_param_usage("id", 36, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: None,
        param_scopes: vec![
            ExpandedParamScope::RepeatItem {
                repeat_id: "ids".to_owned(),
            },
            ExpandedParamScope::RepeatItem {
                repeat_id: "ids".to_owned(),
            },
        ],
        param_occurrences: vec![
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "ids".to_owned(),
                representative_item_index: 1,
                repeat_location: repeat_location.clone(),
            }),
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "ids".to_owned(),
                representative_item_index: 2,
                repeat_location,
            }),
        ],
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("id".to_owned(), core::CoreType::String),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .expect_err("Repeat Param type conflicts should include repeat locations");

    assert_diagnostic_messages(
        &report,
        "conflicting Repeat item Param `id` type in query `findUsers`, Repeat `ids`: first representative occurrence resolved to Int64 but conflicting representative occurrence resolved to String; Repeat item fields with the same ID must resolve matching Param type and nullability\nfirst Repeat `ids` occurrence is here\nconflicting Repeat `ids` occurrence is here",
    );
    assert_eq!(
        report.diagnostics()[1].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[2].location(),
        Some(&expected_repeat_location)
    );
}

#[test]
fn query_repeat_param_validation_rejects_conflicting_enum_values() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/orders.sql",
        core::SourcePosition::one_based(4, 12).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findOrders".to_owned(), None),
        "SELECT id FROM orders WHERE status IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("status", 39, false),
        test_param_usage("status", 41, false),
    ]);
    let variant = AnalyzedQueryVariant {
        query,
        analysis: core::AnalyzedQuery::new(core::Cardinality::Many),
        context: None,
        param_scopes: vec![
            ExpandedParamScope::RepeatItem {
                repeat_id: "statuses".to_owned(),
            },
            ExpandedParamScope::RepeatItem {
                repeat_id: "statuses".to_owned(),
            },
        ],
        param_occurrences: vec![
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "statuses".to_owned(),
                representative_item_index: 1,
                repeat_location: repeat_location.clone(),
            }),
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "statuses".to_owned(),
                representative_item_index: 2,
                repeat_location,
            }),
        ],
    };
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new_type_ref("status".to_owned(), enum_type_ref(["draft", "paid"])),
        core::DbParamUsage::new_type_ref("status".to_owned(), enum_type_ref(["draft", "void"])),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
            .expect_err("Repeat item Param enum value conflicts should be rejected");

    assert_diagnostic_messages(
        &report,
        "conflicting Repeat item Param `status` type in query `findOrders`, Repeat `statuses`: first representative occurrence resolved to Enum([\"draft\", \"paid\"]) but conflicting representative occurrence resolved to Enum([\"draft\", \"void\"]); Repeat item fields with the same ID must resolve matching Param type and nullability\nfirst Repeat `statuses` occurrence is here\nconflicting Repeat `statuses` occurrence is here",
    );
}

#[test]
fn mutation_repeat_param_validation_reports_repeat_locations() {
    let repeat_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(5, 1).expect("test position should be valid"),
    );
    let expected_repeat_location = repeat_location.clone();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        "INSERT INTO users (email) VALUES (?),(?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("email", 34, false),
        test_param_usage("email", 38, true),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Insert),
        context: None,
        param_scopes: vec![
            ExpandedParamScope::RepeatItem {
                repeat_id: "rows".to_owned(),
            },
            ExpandedParamScope::RepeatItem {
                repeat_id: "rows".to_owned(),
            },
        ],
        param_occurrences: vec![
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "rows".to_owned(),
                representative_item_index: 1,
                repeat_location: repeat_location.clone(),
            }),
            ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
                repeat_id: "rows".to_owned(),
                representative_item_index: 2,
                repeat_location,
            }),
        ],
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .expect_err("Repeat Param nullability conflicts should include repeat locations");

    assert_diagnostic_messages(
        &report,
        "conflicting Repeat item Param `email` nullability in mutation `createUsers`, Repeat `rows`: first representative occurrence is nullable false but conflicting representative occurrence is nullable true; Repeat item fields with the same ID must resolve matching Param type and nullability\nfirst Repeat `rows` occurrence is here\nconflicting Repeat `rows` occurrence is here",
    );
    assert_eq!(
        report.diagnostics()[1].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[2].location(),
        Some(&expected_repeat_location)
    );
}

#[test]
fn mutation_fragment_repeat_param_validation_reports_slot_fragment_and_repeat_context() {
    let slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(9, 5).expect("test position should be valid"),
    );
    let repeat_location = core::SourceLocation::at_position(
        "sql/mutation_fragments.sql",
        core::SourcePosition::one_based(2, 17).expect("test position should be valid"),
    );
    let expected_slot_location = slot_location.clone();
    let expected_repeat_location = repeat_location.clone();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("touchUsers".to_owned()),
        "UPDATE users AS u SET name = name WHERE u.id IN (?,?);".to_owned(),
    )
    .with_param_usages(vec![
        test_param_usage("id", 48, false),
        test_param_usage("id", 50, false),
    ]);
    let variant = AnalyzedMutationVariant {
        mutation,
        analysis: core::AnalyzedMutation::new(core::MutationKind::Update),
        context: Some(SlotExpansionContext {
            source_kind: SlotExpansionSourceKind::Mutation,
            source_id: "touchUsers".to_owned(),
            selections: vec![SlotSelectionContext {
                slot_id: "filter".to_owned(),
                target_id: Some("byIds".to_owned()),
                slot_location: slot_location.clone(),
                fragment_location: None,
            }],
        }),
        param_scopes: vec![
            ExpandedParamScope::FragmentRepeatItem {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
            },
            ExpandedParamScope::FragmentRepeatItem {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
            },
        ],
        param_occurrences: vec![
            ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
                representative_item_index: 1,
                slot_occurrence_index: 1,
                slot_location,
                repeat_location: repeat_location.clone(),
            }),
            ExpandedParamOccurrence::FragmentRepeatItem(ExpandedFragmentRepeatParamOccurrence {
                slot_id: "filter".to_owned(),
                target_id: "byIds".to_owned(),
                repeat_id: "ids".to_owned(),
                representative_item_index: 2,
                slot_occurrence_index: 1,
                slot_location: core::SourceLocation::unknown(),
                repeat_location,
            }),
        ],
    };
    let metadata = core::DbMutationMetadata::new().with_param_usages(vec![
        core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("id".to_owned(), core::CoreType::String),
    ]);
    let mut scoped_param_bindings = Vec::new();

    let report = validate_expanded_mutation_variant_param_bindings(
        &variant,
        &metadata,
        &mut scoped_param_bindings,
    )
    .map_err(|report| with_slot_variant_context(report, variant.context.as_ref()))
    .expect_err("mutation Fragment Repeat Param type conflicts should include context");

    assert_diagnostic_messages(
        &report,
        "conflicting Fragment Repeat item Param `id` type in mutation `touchUsers`, Slot `filter`, Fragment `byIds`, Repeat `ids`: first representative occurrence resolved to Int64 but conflicting representative occurrence resolved to String; Repeat item fields with the same ID must resolve matching Param type and nullability\nfirst Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nconflicting Repeat `ids` occurrence in Slot `filter` selecting Fragment `byIds` is here\nwhile validating Slot expansion variant for mutation `touchUsers` with selections: filter=byIds\nSlot `filter` selected `byIds` in this variant",
    );
    assert_eq!(
        report.diagnostics()[1].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[2].location(),
        Some(&expected_repeat_location)
    );
    assert_eq!(
        report.diagnostics()[4].location(),
        Some(&expected_slot_location)
    );
}

#[test]
fn repeat_core_ir_query_param_binding_rejects_missing_placeholder_metadata() {
    let sql = "SELECT id FROM users WHERE id IN (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let repeat = core::RepeatUsage::new(
        "ids".to_owned(),
        ",".to_owned(),
        placeholder,
        placeholder + 1,
        core::SourceLocation::unknown(),
    )
    .with_item_param_usages(vec![core::ParamUsage::new(
        "id".to_owned(),
        None,
        false,
        core::SourceLocation::unknown(),
    )]);
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![repeat]);
    let scoped_param_bindings = vec![repeat_item_binding("ids", "id")];

    let report = compile_dynamic_query_body(&query, &[], &HashMap::new(), &scoped_param_bindings)
        .expect_err("Repeat item placeholder metadata errors should propagate");

    assert_diagnostic_messages(
        &report,
        "Param `id` in Repeat `ids` is missing placeholder position metadata",
    );
}

#[test]
fn repeat_core_ir_mutation_param_binding_rejects_missing_placeholder_metadata() {
    let sql = "INSERT INTO users (email) VALUES (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let repeat = core::RepeatUsage::new(
        "rows".to_owned(),
        ",".to_owned(),
        placeholder - 1,
        placeholder + 2,
        core::SourceLocation::unknown(),
    )
    .with_item_param_usages(vec![core::ParamUsage::new(
        "email".to_owned(),
        None,
        false,
        core::SourceLocation::unknown(),
    )]);
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![repeat]);
    let scoped_param_bindings = vec![repeat_item_binding("rows", "email")];

    let report =
        compile_dynamic_mutation_body(&mutation, &[], &HashMap::new(), &scoped_param_bindings)
            .expect_err("Repeat item placeholder metadata errors should propagate");

    assert_diagnostic_messages(
        &report,
        "Param `email` in Repeat `rows` is missing placeholder position metadata",
    );
}

#[test]
fn repeat_core_ir_query_param_binding_rejects_out_of_range_placeholder_metadata() {
    let sql = "SELECT id FROM users WHERE id IN (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let repeat = core::RepeatUsage::new(
        "ids".to_owned(),
        ",".to_owned(),
        placeholder,
        placeholder + 1,
        core::SourceLocation::unknown(),
    )
    .with_item_param_usages(vec![test_param_usage("id", placeholder + 2, false)]);
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![repeat]);
    let scoped_param_bindings = vec![repeat_item_binding("ids", "id")];

    let report = compile_dynamic_query_body(&query, &[], &HashMap::new(), &scoped_param_bindings)
        .expect_err("Repeat item placeholder range errors should propagate");

    assert_diagnostic_messages(
        &report,
        &format!(
            "Param `id` placeholder index {} is outside Repeat `ids` item range {}..{}",
            placeholder + 2,
            placeholder,
            placeholder + 1
        ),
    );
}

#[test]
fn repeat_core_ir_mutation_param_binding_rejects_out_of_range_placeholder_metadata() {
    let sql = "INSERT INTO users (email) VALUES (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let repeat = core::RepeatUsage::new(
        "rows".to_owned(),
        ",".to_owned(),
        placeholder - 1,
        placeholder + 2,
        core::SourceLocation::unknown(),
    )
    .with_item_param_usages(vec![test_param_usage("email", placeholder + 3, false)]);
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![repeat]);
    let scoped_param_bindings = vec![repeat_item_binding("rows", "email")];

    let report =
        compile_dynamic_mutation_body(&mutation, &[], &HashMap::new(), &scoped_param_bindings)
            .expect_err("Repeat item placeholder range errors should propagate");

    assert_diagnostic_messages(
        &report,
        &format!(
            "Param `email` placeholder index {} is outside Repeat `rows` item range {}..{}",
            placeholder + 3,
            placeholder - 1,
            placeholder + 2
        ),
    );
}

#[test]
fn repeat_core_ir_query_rejects_repeat_crossing_slot_segment_boundary() {
    let sql = "SELECT id FROM users WHERE id IN (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![core::RepeatUsage::new(
        "ids".to_owned(),
        ",".to_owned(),
        placeholder - 1,
        placeholder + 2,
        core::SourceLocation::unknown(),
    )])
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byIds".to_owned()],
        placeholder,
        core::SourceLocation::unknown(),
    )]);

    let report = compile_dynamic_query_body(&query, &[], &HashMap::new(), &[])
        .expect_err("Repeat ranges must not cross Slot Core IR segments");

    assert_diagnostic_messages(
        &report,
        "Repeat `ids` in query `findUsers` crosses a Slot Core IR segment boundary",
    );
}

#[test]
fn repeat_core_ir_mutation_rejects_repeat_crossing_slot_segment_boundary() {
    let sql = "INSERT INTO users (email) VALUES (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![core::RepeatUsage::new(
        "rows".to_owned(),
        ",".to_owned(),
        placeholder - 1,
        placeholder + 2,
        core::SourceLocation::unknown(),
    )])
    .with_slot_usages(vec![core::SlotUsage::new(
        "suffix".to_owned(),
        vec!["returningNothing".to_owned()],
        placeholder,
        core::SourceLocation::unknown(),
    )]);

    let report = compile_dynamic_mutation_body(&mutation, &[], &HashMap::new(), &[])
        .expect_err("Repeat ranges must not cross Slot Core IR segments");

    assert_diagnostic_messages(
        &report,
        "Repeat `rows` in mutation `createUsers` crosses a Slot Core IR segment boundary",
    );
}

fn assert_diagnostic_messages(report: &core::DiagnosticReport, expected: &str) {
    assert_eq!(
        report
            .diagnostics()
            .iter()
            .map(core::Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n"),
        expected
    );
}

fn repeat_item_binding(repeat_id: &str, param_id: &str) -> ScopedParamBinding {
    ScopedParamBinding {
        scope: ExpandedParamScope::RepeatItem {
            repeat_id: repeat_id.to_owned(),
        },
        id: param_id.to_owned(),
        type_ref: core::CoreTypeRef::from(core::CoreType::String),
        nullable: false,
        first_occurrence: ExpandedParamOccurrence::RepeatItem(ExpandedRepeatParamOccurrence {
            repeat_id: repeat_id.to_owned(),
            representative_item_index: 1,
            repeat_location: core::SourceLocation::unknown(),
        }),
    }
}

fn enum_type_ref(values: impl IntoIterator<Item = &'static str>) -> core::CoreTypeRef {
    core::CoreTypeRef::from_enum_values(values.into_iter().map(str::to_owned).collect())
        .expect("test enum values should build a Core type reference")
}

fn test_param_usage(id: &str, placeholder_index: usize, nullable: bool) -> core::ParamUsage {
    core::ParamUsage::new(
        id.to_owned(),
        None,
        nullable,
        core::SourceLocation::unknown(),
    )
    .with_placeholder_index(placeholder_index)
}
