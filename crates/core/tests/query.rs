use std::path::Path;

use sqlay_core::{
    AnalyzedMutation, AnalyzedQuery, Cardinality, CoreType, FragmentMetadata, MutationKind,
    MutationMetadata, ParamUsage, QueryMetadata, RawFragment, RawMutation, RawQuery, RawSourceUnit,
    SlotUsage, SourceLocation, SourcePosition, SourceRange,
};

#[test]
fn raw_query_preserves_metadata_sql_source_path_and_optional_source_location() {
    let location = SourceLocation::at_range(
        "sql/users.sql",
        SourceRange::point(SourcePosition::one_based(8, 1).expect("test position should be valid")),
    );
    let query = RawQuery::new(
        QueryMetadata::new("listUsers".to_owned(), Some(Cardinality::One)),
        "SELECT id FROM users;".to_owned(),
    )
    .with_source_path("sql/users.sql")
    .with_source_location(location.clone());

    assert_eq!(query.metadata().id(), "listUsers");
    assert_eq!(query.metadata().cardinality(), Some(Cardinality::One));
    assert_eq!(query.sql(), "SELECT id FROM users;");
    assert_eq!(query.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(query.source_location(), Some(&location));
}

#[test]
fn raw_query_can_carry_analysis_sql_and_param_usages() {
    let location = SourceLocation::from_range(SourceRange::point(
        SourcePosition::one_based(8, 15).expect("test position should be valid"),
    ));
    let query = RawQuery::new(
        QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE email = /* @sqlay { type: param id: email valueType: string nullable: true } */ 'test@example.test' /* @sqlay { type: paramEnd } */;".to_owned(),
    )
    .with_analysis_sql("SELECT id FROM users WHERE email = ?;".to_owned())
    .with_param_usages(vec![ParamUsage::new(
        "email".to_owned(),
        Some(CoreType::String),
        true,
        location.clone(),
    )]);

    assert_eq!(
        query.analysis_sql(),
        "SELECT id FROM users WHERE email = ?;"
    );
    assert_eq!(query.param_usages().len(), 1);
    assert_eq!(query.param_usages()[0].id(), "email");
    assert_eq!(
        query.param_usages()[0].value_type_override(),
        Some(CoreType::String)
    );
    assert!(query.param_usages()[0].nullable_override());
    assert_eq!(query.param_usages()[0].source_location(), &location);
}

#[test]
fn raw_query_can_carry_slot_usages() {
    let location = SourceLocation::from_range(SourceRange::point(
        SourcePosition::one_based(8, 45).expect("test position should be valid"),
    ));
    let query = RawQuery::new(
        QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT id FROM users WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![SlotUsage::new(
        "filter".to_owned(),
        vec!["activeOnly".to_owned()],
        32,
        location.clone(),
    )]);

    assert_eq!(query.analysis_sql(), "SELECT id FROM users WHERE 1 = 1;");
    assert_eq!(query.slot_usages().len(), 1);
    assert_eq!(query.slot_usages()[0].id(), "filter");
    assert_eq!(query.slot_usages()[0].targets(), ["activeOnly"]);
    assert_eq!(query.slot_usages()[0].insertion_index(), 32);
    assert_eq!(query.slot_usages()[0].source_location(), &location);
}

#[test]
fn raw_fragment_preserves_metadata_sql_source_path_and_optional_source_location() {
    let location = SourceLocation::at_range(
        "sql/fragments.sql",
        SourceRange::point(SourcePosition::one_based(7, 1).expect("test position should be valid")),
    );
    let fragment = RawFragment::new(
        FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1\n".to_owned(),
    )
    .with_source_path("sql/fragments.sql")
    .with_source_location(location.clone());

    assert_eq!(fragment.metadata().id(), "activeOnly");
    assert_eq!(fragment.sql(), "\nAND u.active = 1\n");
    assert_eq!(fragment.source_path(), Some(Path::new("sql/fragments.sql")));
    assert_eq!(fragment.source_location(), Some(&location));
}

#[test]
fn raw_fragment_can_carry_analysis_sql_and_param_usages() {
    let location = SourceLocation::from_range(SourceRange::point(
        SourcePosition::one_based(8, 15).expect("test position should be valid"),
    ));
    let fragment = RawFragment::new(
        FragmentMetadata::new("byEmail".to_owned()),
        "\nAND u.email = /* @sqlay { type: param id: email valueType: string } */ 'ada@example.test' /* @sqlay { type: paramEnd } */\n".to_owned(),
    )
    .with_analysis_sql("\nAND u.email = ?\n".to_owned())
    .with_param_usages(vec![ParamUsage::new(
        "email".to_owned(),
        Some(CoreType::String),
        false,
        location.clone(),
    )]);

    assert_eq!(fragment.analysis_sql(), "\nAND u.email = ?\n");
    assert_eq!(fragment.param_usages().len(), 1);
    assert_eq!(fragment.param_usages()[0].id(), "email");
    assert_eq!(
        fragment.param_usages()[0].value_type_override(),
        Some(CoreType::String)
    );
    assert_eq!(fragment.param_usages()[0].source_location(), &location);
}

#[test]
fn raw_mutation_preserves_metadata_sql_source_path_and_optional_source_location() {
    let location = SourceLocation::at_range(
        "sql/users.sql",
        SourceRange::point(SourcePosition::one_based(8, 1).expect("test position should be valid")),
    );
    let mutation = RawMutation::new(
        MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES ('ada@example.test');".to_owned(),
    )
    .with_source_path("sql/users.sql")
    .with_source_location(location.clone());

    assert_eq!(mutation.metadata().id(), "createUser");
    assert_eq!(
        mutation.sql(),
        "INSERT INTO users (email) VALUES ('ada@example.test');"
    );
    assert_eq!(mutation.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(mutation.source_location(), Some(&location));
}

#[test]
fn raw_mutation_can_carry_analysis_sql_param_usages_and_slot_usages() {
    let param_location = SourceLocation::from_range(SourceRange::point(
        SourcePosition::one_based(8, 45).expect("test position should be valid"),
    ));
    let slot_location = SourceLocation::from_range(SourceRange::point(
        SourcePosition::one_based(9, 1).expect("test position should be valid"),
    ));
    let mutation = RawMutation::new(
        MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (/* @sqlay { type: param id: email valueType: string } */ 'ada@example.test' /* @sqlay { type: paramEnd } */)/* @sqlay { type: slot id: writeMode targets: [upsertName] } */;".to_owned(),
    )
    .with_analysis_sql("INSERT INTO users (email) VALUES (?);".to_owned())
    .with_param_usages(vec![ParamUsage::new(
        "email".to_owned(),
        Some(CoreType::String),
        false,
        param_location.clone(),
    )])
    .with_slot_usages(vec![SlotUsage::new(
        "writeMode".to_owned(),
        vec!["upsertName".to_owned()],
        36,
        slot_location.clone(),
    )]);

    assert_eq!(
        mutation.analysis_sql(),
        "INSERT INTO users (email) VALUES (?);"
    );
    assert_eq!(mutation.param_usages().len(), 1);
    assert_eq!(mutation.param_usages()[0].id(), "email");
    assert_eq!(
        mutation.param_usages()[0].value_type_override(),
        Some(CoreType::String)
    );
    assert_eq!(
        mutation.param_usages()[0].source_location(),
        &param_location
    );
    assert_eq!(mutation.slot_usages().len(), 1);
    assert_eq!(mutation.slot_usages()[0].id(), "writeMode");
    assert_eq!(mutation.slot_usages()[0].targets(), ["upsertName"]);
    assert_eq!(mutation.slot_usages()[0].insertion_index(), 36);
    assert_eq!(mutation.slot_usages()[0].source_location(), &slot_location);
}

#[test]
fn raw_source_unit_wraps_query_mutation_and_fragment_units() {
    let query = RawQuery::new(
        QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users;".to_owned(),
    );
    let mutation = RawMutation::new(
        MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES ('ada@example.test');".to_owned(),
    );
    let fragment = RawFragment::new(
        FragmentMetadata::new("activeOnly".to_owned()),
        "AND u.active = 1".to_owned(),
    );

    let source_units = [
        RawSourceUnit::Query(query),
        RawSourceUnit::Mutation(mutation),
        RawSourceUnit::Fragment(fragment),
    ];

    assert_eq!(source_units[0].id(), "listUsers");
    assert_eq!(source_units[1].id(), "createUser");
    assert_eq!(source_units[2].id(), "activeOnly");
}

#[test]
fn analyzed_query_exposes_inferred_cardinality() {
    let analysis = AnalyzedQuery::new(Cardinality::Many);

    assert_eq!(analysis.cardinality(), Cardinality::Many);
}

#[test]
fn analyzed_mutation_exposes_statement_kind() {
    let analysis = AnalyzedMutation::new(MutationKind::Update);

    assert_eq!(analysis.kind(), MutationKind::Update);
}
