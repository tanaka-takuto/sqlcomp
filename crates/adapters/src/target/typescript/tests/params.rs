use sqlay_core as core;

use super::super::render_query;

#[test]
fn renders_single_param_query_with_required_input_and_tuple_param() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("findCustomerByEmail".to_owned()),
        "SELECT id FROM customers WHERE email = ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![core::ParamBinding::new(
        "email".to_owned(),
        core::CoreType::String,
        false,
    )]);

    assert_eq!(
        render_query(&query),
        r#"export type findCustomerByEmail_Input = {
  email: string;
};

export type findCustomerByEmail_Row = {
  id: string;
};

export type findCustomerByEmail_Output = findCustomerByEmail_Row[];

export function findCustomerByEmail(
  input: findCustomerByEmail_Input,
): { sql: string; params: readonly [string] } {
  return {
    sql: "SELECT id FROM customers WHERE email = ?;",
    params: [input.email] as const,
  };
}
"#
    );
}

#[test]
fn renders_multiple_repeated_and_nullable_params_in_usage_order() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("findCustomerActivity".to_owned()),
        "SELECT id FROM customers WHERE email = ? OR backup_email = ? OR created_at >= ? OR rank <= ?;".to_owned(),
        core::Cardinality::Many,
        vec![
            core::InputField::new("email".to_owned(), core::CoreType::String, false),
            core::InputField::new("since".to_owned(), core::CoreType::DateTime, true),
            core::InputField::new("maxRank".to_owned(), core::CoreType::Int32, false),
        ],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![
        core::ParamBinding::new("email".to_owned(), core::CoreType::String, false),
        core::ParamBinding::new("email".to_owned(), core::CoreType::String, false),
        core::ParamBinding::new("since".to_owned(), core::CoreType::DateTime, true),
        core::ParamBinding::new("maxRank".to_owned(), core::CoreType::Int32, false),
    ]);

    assert_eq!(
        render_query(&query),
        r#"export type findCustomerActivity_Input = {
  email: string;
  since: string | null;
  maxRank: number;
};

export type findCustomerActivity_Row = {
  id: string;
};

export type findCustomerActivity_Output = findCustomerActivity_Row[];

export function findCustomerActivity(
  input: findCustomerActivity_Input,
): { sql: string; params: readonly [string, string, string | null, number] } {
  return {
    sql: "SELECT id FROM customers WHERE email = ? OR backup_email = ? OR created_at >= ? OR rank <= ?;",
    params: [input.email, input.email, input.since, input.maxRank] as const,
  };
}
"#
    );
}

#[test]
fn renders_param_expression_with_safe_property_access() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("findCustomer".to_owned()),
        "SELECT id FROM customers WHERE email = ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "customer email".to_owned(),
            core::CoreType::String,
            false,
        )],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![core::ParamBinding::new(
        "customer email".to_owned(),
        core::CoreType::String,
        false,
    )]);

    let rendered = render_query(&query);

    assert!(rendered.contains("  \"customer email\": string;"));
    assert!(rendered.contains(r#"params: [input["customer email"]] as const,"#));
}

#[test]
fn renders_param_types_with_existing_core_type_mapping() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("inspectParamTypes".to_owned()),
        "SELECT id FROM fixture_types WHERE active = ? AND small_count = ? AND large_count = ? AND ratio = ? AND amount = ? AND payload = ? AND birth_date = ? AND delivery_window = ? AND created_at = ? AND settings = ? AND shape = ?;".to_owned(),
        core::Cardinality::Many,
        vec![
            core::InputField::new("active".to_owned(), core::CoreType::Bool, false),
            core::InputField::new("smallCount".to_owned(), core::CoreType::Int32, false),
            core::InputField::new("largeCount".to_owned(), core::CoreType::Int64, false),
            core::InputField::new("ratio".to_owned(), core::CoreType::Float64, false),
            core::InputField::new("amount".to_owned(), core::CoreType::Decimal, false),
            core::InputField::new("payload".to_owned(), core::CoreType::Bytes, false),
            core::InputField::new("birthDate".to_owned(), core::CoreType::Date, false),
            core::InputField::new("deliveryWindow".to_owned(), core::CoreType::Time, false),
            core::InputField::new("createdAt".to_owned(), core::CoreType::DateTime, false),
            core::InputField::new("settings".to_owned(), core::CoreType::Json, true),
            core::InputField::new("shape".to_owned(), core::CoreType::Unknown, true),
        ],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![
        core::ParamBinding::new("active".to_owned(), core::CoreType::Bool, false),
        core::ParamBinding::new("smallCount".to_owned(), core::CoreType::Int32, false),
        core::ParamBinding::new("largeCount".to_owned(), core::CoreType::Int64, false),
        core::ParamBinding::new("ratio".to_owned(), core::CoreType::Float64, false),
        core::ParamBinding::new("amount".to_owned(), core::CoreType::Decimal, false),
        core::ParamBinding::new("payload".to_owned(), core::CoreType::Bytes, false),
        core::ParamBinding::new("birthDate".to_owned(), core::CoreType::Date, false),
        core::ParamBinding::new("deliveryWindow".to_owned(), core::CoreType::Time, false),
        core::ParamBinding::new("createdAt".to_owned(), core::CoreType::DateTime, false),
        core::ParamBinding::new("settings".to_owned(), core::CoreType::Json, true),
        core::ParamBinding::new("shape".to_owned(), core::CoreType::Unknown, true),
    ]);

    let rendered = render_query(&query);

    assert!(rendered.contains(
        r"export type inspectParamTypes_Input = {
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
};"
    ));
    assert!(rendered.contains(
        "params: readonly [boolean, number, string, number, string, Uint8Array, string, string, string, unknown | null, unknown | null]"
    ));
}
