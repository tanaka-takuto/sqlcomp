use super::support::*;
use super::*;

#[test]
fn query_compiler_builds_core_ir_with_empty_paramless_input_and_result_columns() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id, name FROM users;".to_owned(),
    )
    .with_source_path("sql/users.sql");
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(vec![
        core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
        core::DbResultColumn::new("name".to_owned(), core::CoreType::String, Some(true)),
    ]);

    let compiled = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect("query should compile into core IR");

    assert_eq!(compiled.id().as_str(), "listUsers");
    assert_eq!(compiled.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(compiled.sql(), "SELECT id, name FROM users;");
    assert_eq!(compiled.cardinality(), core::Cardinality::Many);
    assert!(compiled.input().is_empty());
    assert_eq!(compiled.row().len(), 2);
    assert_eq!(compiled.row()[0].name(), "id");
    assert_eq!(compiled.row()[0].ty(), core::CoreType::Int64);
    assert!(!compiled.row()[0].is_nullable());
    assert_eq!(compiled.row()[1].name(), "name");
    assert_eq!(compiled.row()[1].ty(), core::CoreType::String);
    assert!(compiled.row()[1].is_nullable());
}

#[test]
fn query_compiler_builds_input_fields_and_param_bindings_from_resolved_param_metadata() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE email = /* @sqlcomp { type: param id: email nullable: true } */ 'test@example.test' /* @sqlcomp { type: paramEnd } */ AND id = /* @sqlcomp { type: param id: userId } */ 1 /* @sqlcomp { type: paramEnd } */ OR email = /* @sqlcomp { type: param id: email nullable: true } */ 'ada@example.test' /* @sqlcomp { type: paramEnd } */;".to_owned(),
    )
    .with_analysis_sql("SELECT id FROM users WHERE email = ? AND id = ? OR email = ?;".to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);

    let compiled = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect("resolved Param query should compile into Core IR");

    assert_eq!(
        compiled.input(),
        [
            core::InputField::new("email".to_owned(), core::CoreType::String, true),
            core::InputField::new("userId".to_owned(), core::CoreType::Int64, false),
        ]
    );
    assert_eq!(
        compiled.params(),
        [
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, true),
            core::ParamBinding::new("userId".to_owned(), core::CoreType::Int64, false),
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, true),
        ]
    );
}

#[test]
fn query_compiler_rejects_repeated_param_ids_with_conflicting_semantics() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE id = ? OR id = ?;".to_owned(),
    )
    .with_param_usages(vec![
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::String),
    ]);

    let report = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect_err("conflicting repeated Param IDs should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `userId` types: first occurrence resolved to Int64 but later occurrence resolved to String"
    );
}

#[test]
fn query_compiler_rejects_repeated_param_ids_with_conflicting_nullability() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE email = ? OR email = ?;".to_owned(),
    )
    .with_param_usages(vec![
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);

    let report = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect_err("conflicting repeated Param nullability should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `email` nullability: first occurrence is nullable false but later occurrence is nullable true"
    );
}

#[test]
fn query_compiler_uses_inferred_cardinality_when_metadata_has_no_override() {
    let compiled = compile_query(None, core::Cardinality::Many);

    assert_eq!(compiled.cardinality(), core::Cardinality::Many);
}

#[test]
fn query_compiler_uses_explicit_one_cardinality_over_inference() {
    let compiled = compile_query(Some(core::Cardinality::One), core::Cardinality::Many);

    assert_eq!(compiled.cardinality(), core::Cardinality::One);
}

#[test]
fn query_compiler_uses_explicit_many_cardinality_over_inference() {
    let compiled = compile_query(Some(core::Cardinality::Many), core::Cardinality::One);

    assert_eq!(compiled.cardinality(), core::Cardinality::Many);
}

#[test]
fn query_compiler_copies_database_columns_to_result_row() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id, nickname FROM users;".to_owned(),
    );
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(vec![
        core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
        core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true)),
    ]);

    let compiled = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect("query compiler should preserve result row metadata");

    assert_eq!(
        compiled.row(),
        [
            core::ResultColumn::new("id".to_owned(), core::CoreType::Int64, false),
            core::ResultColumn::new("nickname".to_owned(), core::CoreType::String, true),
        ]
    );
}

#[test]
fn query_compiler_maps_unknown_nullability_to_nullable_result_row() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("inspectUsers".to_owned(), None),
        "SELECT id, nickname, computed_name FROM users;".to_owned(),
    );
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(vec![
        core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
        core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true)),
        core::DbResultColumn::new("computed_name".to_owned(), core::CoreType::String, None),
    ]);

    let compiled = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect("query compiler should preserve conservative nullability");

    assert_eq!(
        compiled.row(),
        [
            core::ResultColumn::new("id".to_owned(), core::CoreType::Int64, false),
            core::ResultColumn::new("nickname".to_owned(), core::CoreType::String, true),
            core::ResultColumn::new("computed_name".to_owned(), core::CoreType::String, true),
        ]
    );
}
