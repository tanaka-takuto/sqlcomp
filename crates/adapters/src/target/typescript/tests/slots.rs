use sqlcomp_core as core;

use super::super::{render_generated_file_contents, render_query};
use super::support::{compiled_query, param, slot_branch, slot_definition, sql_segment};

const SLOT_QUERY_RUNTIME_BRANCHES: &str = r#"export type listUsers_Input = {
  status: string;
  filter?: { $fragment: "activeOnly" } | {
    $fragment: "byEmail";
    email: string;
  } | {
    $fragment: "createdSince";
    since: string | null;
  };
  sort?: { $fragment: "orderByName" };
};

export type listUsers_Row = {
  id: string;
};

export type listUsers_Output = listUsers_Row[];

export function listUsers(
  input: listUsers_Input,
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  sqlParts.push("SELECT id FROM users WHERE status = ?");
  params.push(input.status);
  switch (input.filter?.$fragment) {
    case "activeOnly":
      sqlParts.push(" AND active = 1");
      break;
    case "byEmail":
      sqlParts.push(" AND email = ?");
      params.push(input.filter.email);
      break;
    case "createdSince":
      sqlParts.push(" AND created_at >= ?");
      params.push(input.filter.since);
      break;
  }
  sqlParts.push(" ");
  switch (input.sort?.$fragment) {
    case "orderByName":
      sqlParts.push(" ORDER BY name");
      break;
  }
  sqlParts.push(";");

  return {
    sql: sqlParts.join(""),
    params,
  };
}
"#;

#[test]
fn renders_slot_query_runtime_branches_with_params_in_sql_order() {
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment(
                "SELECT id FROM users WHERE status = ?",
                vec![param("status", core::CoreType::String, false)],
            ),
            sql_segment(" ", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![
            core::CompiledSlotOccurrence::new("filter".to_owned()),
            core::CompiledSlotOccurrence::new("sort".to_owned()),
        ],
        vec![
            slot_definition(
                "filter",
                vec![
                    slot_branch("activeOnly", " AND active = 1", Vec::new()),
                    slot_branch(
                        "byEmail",
                        " AND email = ?",
                        vec![param("email", core::CoreType::String, false)],
                    ),
                    slot_branch(
                        "createdSince",
                        " AND created_at >= ?",
                        vec![param("since", core::CoreType::DateTime, true)],
                    ),
                ],
            ),
            slot_definition(
                "sort",
                vec![slot_branch("orderByName", " ORDER BY name", Vec::new())],
            ),
        ],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("listUsers".to_owned()),
        "SELECT id FROM users WHERE status = ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "status".to_owned(),
            core::CoreType::String,
            false,
        )],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![param("status", core::CoreType::String, false)])
    .with_dynamic_body(dynamic_body);

    assert_eq!(render_query(&query), SLOT_QUERY_RUNTIME_BRANCHES);
}

#[test]
fn renders_repeated_slot_runtime_branches_from_one_slot_input() {
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment(
                "SELECT id FROM users WHERE tenant_id = ?",
                vec![param("tenantId", core::CoreType::String, false)],
            ),
            sql_segment(
                " AND status = ? AND EXISTS (SELECT 1 FROM role_links AS rl WHERE rl.user_id = users.id",
                vec![param("status", core::CoreType::String, false)],
            ),
            sql_segment(");", Vec::new()),
        ],
        vec![
            core::CompiledSlotOccurrence::new("filter".to_owned()),
            core::CompiledSlotOccurrence::new("filter".to_owned()),
        ],
        vec![slot_definition(
            "filter",
            vec![slot_branch(
                "byRole",
                " AND (role_id = ? OR fallback_role_id = ?)",
                vec![
                    param("roleId", core::CoreType::String, false),
                    param("roleId", core::CoreType::String, false),
                ],
            )],
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("searchUsers".to_owned()),
        "SELECT id FROM users WHERE tenant_id = ? AND status = ?;".to_owned(),
        core::Cardinality::Many,
        vec![
            core::InputField::new("tenantId".to_owned(), core::CoreType::String, false),
            core::InputField::new("status".to_owned(), core::CoreType::String, false),
        ],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![
        param("tenantId", core::CoreType::String, false),
        param("status", core::CoreType::String, false),
    ])
    .with_dynamic_body(dynamic_body);

    assert_eq!(
        render_query(&query),
        r#"export type searchUsers_Input = {
  tenantId: string;
  status: string;
  filter?: {
    $fragment: "byRole";
    roleId: string;
  };
};

export type searchUsers_Row = {
  id: string;
};

export type searchUsers_Output = searchUsers_Row[];

export function searchUsers(
  input: searchUsers_Input,
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  sqlParts.push("SELECT id FROM users WHERE tenant_id = ?");
  params.push(input.tenantId);
  switch (input.filter?.$fragment) {
    case "byRole":
      sqlParts.push(" AND (role_id = ? OR fallback_role_id = ?)");
      params.push(input.filter.roleId);
      params.push(input.filter.roleId);
      break;
  }
  sqlParts.push(" AND status = ? AND EXISTS (SELECT 1 FROM role_links AS rl WHERE rl.user_id = users.id");
  params.push(input.status);
  switch (input.filter?.$fragment) {
    case "byRole":
      sqlParts.push(" AND (role_id = ? OR fallback_role_id = ?)");
      params.push(input.filter.roleId);
      params.push(input.filter.roleId);
      break;
  }
  sqlParts.push(");");

  return {
    sql: sqlParts.join(""),
    params,
  };
}
"#
    );
}

#[test]
fn renders_slot_only_query_input_with_empty_object_default() {
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment("SELECT id FROM users WHERE 1 = 1", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![slot_branch(
                "byEmail",
                " AND email = ?",
                vec![param("email", core::CoreType::String, false)],
            )],
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("searchUsers".to_owned()),
        "SELECT id FROM users WHERE 1 = 1;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_dynamic_body(dynamic_body);

    assert_eq!(
        render_query(&query),
        r#"export type searchUsers_Input = {
  filter?: {
    $fragment: "byEmail";
    email: string;
  };
};

export type searchUsers_Row = {
  id: string;
};

export type searchUsers_Output = searchUsers_Row[];

export function searchUsers(
  _input: searchUsers_Input = {},
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  sqlParts.push("SELECT id FROM users WHERE 1 = 1");
  switch (_input.filter?.$fragment) {
    case "byEmail":
      sqlParts.push(" AND email = ?");
      params.push(_input.filter.email);
      break;
  }
  sqlParts.push(";");

  return {
    sql: sqlParts.join(""),
    params,
  };
}
"#
    );
}

#[test]
fn generated_file_with_slot_query_includes_private_sql_param_alias() {
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment("SELECT id FROM users WHERE 1 = 1", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![slot_branch("activeOnly", " AND active = 1", Vec::new())],
        )],
    );
    let slot_query = compiled_query("listUsers", "SELECT id FROM users WHERE 1 = 1;")
        .with_dynamic_body(dynamic_body);
    let static_query = compiled_query("listRoles", "SELECT id FROM roles;");

    let contents = render_generated_file_contents(&[slot_query, static_query]);

    assert!(contents.starts_with(
        "// @generated by sqlcomp. Do not edit.\n\n\
type SqlParam = unknown;\n\n\
export type listUsers_Input"
    ));
    assert_eq!(contents.matches("type SqlParam = unknown;").count(), 1);
}
