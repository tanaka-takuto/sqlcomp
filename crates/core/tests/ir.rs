use std::path::Path;

use sqlay_core::{
    Cardinality, CompiledBuilder, CompiledDynamicQuery, CompiledMutation, CompiledQuery,
    CompiledRepeatOccurrence, CompiledSlotBranch, CompiledSlotDefinition, CompiledSlotOccurrence,
    CompiledSqlBody, CompiledSqlSegment, CoreType, InputField, MutationId, MutationKind,
    ParamBinding, QueryId, ResultColumn,
};

#[test]
fn compiled_query_represents_empty_paramless_input_and_result_columns() {
    let query = CompiledQuery::new(
        QueryId::new("listUsers".to_owned()),
        "SELECT id, name FROM users;".to_owned(),
        Cardinality::Many,
        Vec::new(),
        vec![
            ResultColumn::new("id".to_owned(), CoreType::Int64, false),
            ResultColumn::new("name".to_owned(), CoreType::String, true),
        ],
    );

    assert_eq!(query.id().as_str(), "listUsers");
    assert_eq!(query.sql(), "SELECT id, name FROM users;");
    assert_eq!(query.cardinality(), Cardinality::Many);
    assert_eq!(query.source_path(), None);
    assert!(query.input().is_empty());
    assert!(query.params().is_empty());
    assert_eq!(query.row().len(), 2);
    assert_eq!(query.row()[0].name(), "id");
    assert_eq!(query.row()[0].ty(), CoreType::Int64);
    assert!(!query.row()[0].is_nullable());
    assert_eq!(query.row()[1].name(), "name");
    assert_eq!(query.row()[1].ty(), CoreType::String);
    assert!(query.row()[1].is_nullable());
}

#[test]
fn compiled_query_preserves_source_path_when_available() {
    let query = CompiledQuery::new(
        QueryId::new("listUsers".to_owned()),
        "SELECT id FROM users;".to_owned(),
        Cardinality::Many,
        Vec::new(),
        Vec::new(),
    )
    .with_source_path("sql/users.sql");

    assert_eq!(query.source_path(), Some(Path::new("sql/users.sql")));
}

#[test]
fn compiled_query_can_carry_dynamic_slot_body() {
    let dynamic_body = CompiledDynamicQuery::new(
        vec![
            CompiledSqlSegment::new(
                "SELECT id FROM users WHERE active = ?".to_owned(),
                vec![ParamBinding::new(
                    "active".to_owned(),
                    CoreType::Bool,
                    false,
                )],
            ),
            CompiledSqlSegment::new(";".to_owned(), Vec::new()),
        ],
        vec![CompiledSlotOccurrence::new("filter".to_owned())],
        vec![CompiledSlotDefinition::new(
            "filter".to_owned(),
            vec![CompiledSlotBranch::new(
                "byEmail".to_owned(),
                vec![CompiledSqlSegment::new(
                    " AND email = ?".to_owned(),
                    vec![ParamBinding::new(
                        "email".to_owned(),
                        CoreType::String,
                        false,
                    )],
                )],
            )],
        )],
    );
    let query = CompiledQuery::new(
        QueryId::new("listUsers".to_owned()),
        "SELECT id FROM users WHERE active = ?;".to_owned(),
        Cardinality::Many,
        Vec::new(),
        Vec::new(),
    )
    .with_dynamic_body(dynamic_body);

    let dynamic_body = query
        .dynamic_body()
        .expect("dynamic body should be present");

    assert_eq!(dynamic_body.base_segments().len(), 2);
    assert_eq!(dynamic_body.slot_occurrences()[0].slot_id(), "filter");
    assert_eq!(dynamic_body.slots()[0].branches()[0].target_id(), "byEmail");
}

#[test]
fn compiled_dynamic_query_legacy_segments_include_repeat_item_sql_and_params() {
    let dynamic_body = CompiledDynamicQuery::new_with_bodies(
        vec![CompiledSqlBody::new(
            vec![
                CompiledSqlSegment::new("SELECT id FROM users WHERE id IN (".to_owned(), vec![]),
                CompiledSqlSegment::new(");".to_owned(), vec![]),
            ],
            vec![CompiledRepeatOccurrence::new(
                "ids".to_owned(),
                ",".to_owned(),
                CompiledSqlSegment::new(
                    "?".to_owned(),
                    vec![ParamBinding::new("id".to_owned(), CoreType::Int64, false)],
                ),
            )],
        )],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    assert_eq!(dynamic_body.base_segments().len(), 1);
    assert_eq!(
        dynamic_body.base_segments()[0].sql(),
        "SELECT id FROM users WHERE id IN (?);"
    );
    assert_eq!(
        dynamic_body.base_segments()[0].params(),
        [ParamBinding::new("id".to_owned(), CoreType::Int64, false)]
    );
}

#[test]
fn compiled_slot_branch_static_body_normalizes_legacy_segments() {
    let branch = CompiledSlotBranch::new(
        "byName".to_owned(),
        vec![
            CompiledSqlSegment::new(
                " AND first_name = ?".to_owned(),
                vec![ParamBinding::new(
                    "firstName".to_owned(),
                    CoreType::String,
                    false,
                )],
            ),
            CompiledSqlSegment::new(
                " AND last_name = ?".to_owned(),
                vec![ParamBinding::new(
                    "lastName".to_owned(),
                    CoreType::String,
                    false,
                )],
            ),
        ],
    );

    assert_eq!(branch.segments().len(), 2);
    assert_eq!(branch.body().base_segments().len(), 1);
    assert_eq!(
        branch.body().base_segments()[0].sql(),
        " AND first_name = ? AND last_name = ?"
    );
    assert_eq!(
        branch.body().base_segments()[0].params(),
        [
            ParamBinding::new("firstName".to_owned(), CoreType::String, false),
            ParamBinding::new("lastName".to_owned(), CoreType::String, false),
        ]
    );
}

#[test]
fn compiled_mutation_represents_input_params_kind_and_source_path_without_result_row() {
    let mutation = CompiledMutation::new(
        MutationId::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (?);".to_owned(),
        MutationKind::Insert,
        vec![InputField::new("email".to_owned(), CoreType::String, false)],
    )
    .with_params(vec![ParamBinding::new(
        "email".to_owned(),
        CoreType::String,
        false,
    )])
    .with_source_path("sql/users.sql");

    assert_eq!(mutation.id().as_str(), "createUser");
    assert_eq!(mutation.sql(), "INSERT INTO users (email) VALUES (?);");
    assert_eq!(mutation.kind(), MutationKind::Insert);
    assert_eq!(mutation.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(mutation.input().len(), 1);
    assert_eq!(mutation.input()[0].name(), "email");
    assert_eq!(mutation.params().len(), 1);
    assert_eq!(mutation.params()[0].input_name(), "email");
}

#[test]
fn compiled_builder_wraps_queries_and_mutations_in_source_order() {
    let query = CompiledQuery::new(
        QueryId::new("listUsers".to_owned()),
        "SELECT id FROM users;".to_owned(),
        Cardinality::Many,
        Vec::new(),
        Vec::new(),
    );
    let mutation = CompiledMutation::new(
        MutationId::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (?);".to_owned(),
        MutationKind::Insert,
        Vec::new(),
    );

    let builders = [
        CompiledBuilder::Query(query),
        CompiledBuilder::Mutation(mutation),
    ];

    assert_eq!(builders[0].id(), "listUsers");
    assert_eq!(builders[1].id(), "createUser");
}
