use super::diagnostics::with_slot_variant_context;
use super::param_validation::validate_expanded_variant_param_bindings;
use super::slot_variants::{
    AnalyzedQueryVariant, ExpandedFragmentParamOccurrence, ExpandedParamOccurrence,
    ExpandedParamScope, SlotExpansionContext, SlotSelectionContext,
};
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

    let report =
        validate_expanded_variant_param_bindings(&variant, &metadata, &mut scoped_param_bindings)
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
