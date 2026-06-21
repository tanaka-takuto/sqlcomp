use sqlcomp_core as core;

use super::super::{render_generated_file_contents, render_query};

#[test]
fn renders_input_row_output_types_and_builder_for_many_cardinality() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("listUsers".to_owned()),
        "SELECT id, name FROM users;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![
            core::ResultColumn::new("id".to_owned(), core::CoreType::Int32, false),
            core::ResultColumn::new("name".to_owned(), core::CoreType::String, true),
        ],
    );

    assert_eq!(
        render_query(&query),
        r#"export type listUsers_Input = Record<string, never>;

export type listUsers_Row = {
  id: number;
  name: string | null;
};

export type listUsers_Output = listUsers_Row[];

export function listUsers(
  _input: listUsers_Input = {},
): { sql: string; params: readonly [] } {
  return {
    sql: "SELECT id, name FROM users;",
    params: [] as const,
  };
}
"#
    );
}

#[test]
fn renders_one_cardinality_output_as_row_or_null() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("findLatestUser".to_owned()),
        "SELECT id FROM users ORDER BY id DESC LIMIT 1;".to_owned(),
        core::Cardinality::One,
        Vec::new(),
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    );

    assert!(
        render_query(&query)
            .contains("export type findLatestUser_Output = findLatestUser_Row | null;")
    );
}

#[test]
fn renders_precision_sensitive_and_unknown_types_conservatively() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("inspectTypes".to_owned()),
        "SELECT * FROM fixture_types;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![
            core::ResultColumn::new("active".to_owned(), core::CoreType::Bool, false),
            core::ResultColumn::new("smallCount".to_owned(), core::CoreType::Int32, false),
            core::ResultColumn::new("largeCount".to_owned(), core::CoreType::Int64, false),
            core::ResultColumn::new("ratio".to_owned(), core::CoreType::Float64, false),
            core::ResultColumn::new("amount".to_owned(), core::CoreType::Decimal, false),
            core::ResultColumn::new("payload".to_owned(), core::CoreType::Bytes, false),
            core::ResultColumn::new("birthDate".to_owned(), core::CoreType::Date, false),
            core::ResultColumn::new("deliveryWindow".to_owned(), core::CoreType::Time, false),
            core::ResultColumn::new("createdAt".to_owned(), core::CoreType::DateTime, false),
            core::ResultColumn::new("settings".to_owned(), core::CoreType::Json, true),
            core::ResultColumn::new("shape".to_owned(), core::CoreType::Unknown, true),
        ],
    );

    assert_eq!(
        render_query(&query),
        r#"export type inspectTypes_Input = Record<string, never>;

export type inspectTypes_Row = {
  active: boolean;
  smallCount: number;
  largeCount: string;
  ratio: number;
  amount: string;
  payload: Uint8Array;
  birthDate: string;
  deliveryWindow: string;
  createdAt: string;
  settings: unknown | null;
  shape: unknown | null;
};

export type inspectTypes_Output = inspectTypes_Row[];

export function inspectTypes(
  _input: inspectTypes_Input = {},
): { sql: string; params: readonly [] } {
  return {
    sql: "SELECT * FROM fixture_types;",
    params: [] as const,
  };
}
"#
    );
}

#[test]
fn renders_result_column_names_as_typescript_property_names_without_transforming() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("selectOddColumns".to_owned()),
        "SELECT 1 AS `user id`, 2 AS `class`;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![
            core::ResultColumn::new("user id".to_owned(), core::CoreType::Int32, false),
            core::ResultColumn::new("class".to_owned(), core::CoreType::String, false),
        ],
    );

    assert!(render_query(&query).contains("  \"user id\": number;\n  class: string;"));
}

#[test]
fn renders_generated_file_header_and_multiple_queries() {
    let queries = [
        core::CompiledQuery::new(
            core::QueryId::new("listUsers".to_owned()),
            "SELECT id FROM users;".to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            vec![core::ResultColumn::new(
                "id".to_owned(),
                core::CoreType::Int32,
                false,
            )],
        ),
        core::CompiledQuery::new(
            core::QueryId::new("findLatestUser".to_owned()),
            "SELECT id FROM users LIMIT 1;".to_owned(),
            core::Cardinality::One,
            Vec::new(),
            vec![core::ResultColumn::new(
                "id".to_owned(),
                core::CoreType::Int32,
                false,
            )],
        ),
    ];

    let contents = render_generated_file_contents(&queries);

    assert!(contents.starts_with("// @generated by sqlcomp. Do not edit.\n\n"));
    assert!(contents.contains("export type listUsers_Output = listUsers_Row[];"));
    assert!(contents.contains("export type findLatestUser_Output = findLatestUser_Row | null;"));
}
