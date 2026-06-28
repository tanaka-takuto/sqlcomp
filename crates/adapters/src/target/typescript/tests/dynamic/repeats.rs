use sqlay_core as core;

use super::super::super::builders::render_mutation;
use super::super::super::{render_generated_file_contents, render_query};
use super::super::support::{compiled_query, param, slot_definition, sql_segment};

#[test]
fn renders_query_repeat_inputs_guards_and_runtime_loops() {
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![core::CompiledSqlBody::new(
            vec![
                sql_segment("SELECT id FROM users WHERE id IN (", Vec::new()),
                sql_segment(");", Vec::new()),
            ],
            vec![core::CompiledRepeatOccurrence::new(
                "ids".to_owned(),
                ",".to_owned(),
                sql_segment("?", vec![param("id", core::CoreType::Int64, false)]),
            )],
        )],
        Vec::new(),
        Vec::new(),
        vec![core::CompiledRepeatDefinition::new(
            "ids".to_owned(),
            vec![param("id", core::CoreType::Int64, false)],
        )],
    );
    let query = compiled_query("findUsersByIds", "SELECT id FROM users WHERE id IN (?);")
        .with_dynamic_body(dynamic_body);

    assert_eq!(
        render_query(&query),
        r#"export type findUsersByIds_Input = {
  ids: readonly [{ id: string }, ...{ id: string }[]];
};

export type findUsersByIds_Row = {
  id: number;
};

export type findUsersByIds_Output = findUsersByIds_Row[];

export function findUsersByIds(
  input: findUsersByIds_Input,
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  if (input.ids.length === 0) {
    throw new Error("Repeat `ids` requires at least one item");
  }

  sqlParts.push("SELECT id FROM users WHERE id IN (");
  {
    let idsIndex = 0;
    for (const idsItem of input.ids) {
      if (idsIndex > 0) {
        sqlParts.push(",");
      }
      sqlParts.push("?");
      params.push(idsItem.id);
      idsIndex += 1;
    }
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
fn renders_mutation_repeat_item_fields_and_params_in_occurrence_order() {
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![core::CompiledSqlBody::new(
            vec![
                sql_segment("INSERT INTO users (email, name) VALUES ", Vec::new()),
                sql_segment(";", Vec::new()),
            ],
            vec![core::CompiledRepeatOccurrence::new(
                "rows".to_owned(),
                ",".to_owned(),
                sql_segment(
                    "(?, ?)",
                    vec![
                        param("name", core::CoreType::String, false),
                        param("email", core::CoreType::String, false),
                    ],
                ),
            )],
        )],
        Vec::new(),
        Vec::new(),
        vec![core::CompiledRepeatDefinition::new(
            "rows".to_owned(),
            vec![
                param("email", core::CoreType::String, false),
                param("name", core::CoreType::String, false),
            ],
        )],
    );
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("createUsers".to_owned()),
        "INSERT INTO users (email, name) VALUES (?, ?);".to_owned(),
        core::MutationKind::Insert,
        Vec::new(),
    )
    .with_dynamic_body(dynamic_body);

    assert_eq!(
        render_mutation(&mutation),
        r#"export type createUsers_Input = {
  rows: readonly [{ email: string; name: string }, ...{ email: string; name: string }[]];
};

export function createUsers(
  input: createUsers_Input,
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  if (input.rows.length === 0) {
    throw new Error("Repeat `rows` requires at least one item");
  }

  sqlParts.push("INSERT INTO users (email, name) VALUES ");
  {
    let rowsIndex = 0;
    for (const rowsItem of input.rows) {
      if (rowsIndex > 0) {
        sqlParts.push(",");
      }
      sqlParts.push("(?, ?)");
      params.push(rowsItem.name);
      params.push(rowsItem.email);
      rowsIndex += 1;
    }
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
fn renders_slot_branch_repeat_guard_only_inside_selected_branch() {
    let branch_body = core::CompiledSqlBody::new(
        vec![
            sql_segment(" AND id IN (", Vec::new()),
            sql_segment(")", Vec::new()),
        ],
        vec![core::CompiledRepeatOccurrence::new(
            "ids".to_owned(),
            ",".to_owned(),
            sql_segment("?", vec![param("id", core::CoreType::Int64, false)]),
        )],
    );
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![
            core::CompiledSqlBody::from_segment(sql_segment(
                "SELECT id FROM users WHERE 1 = 1",
                Vec::new(),
            )),
            core::CompiledSqlBody::from_segment(sql_segment(";", Vec::new())),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![core::CompiledSlotBranch::new_with_body(
                "byIds".to_owned(),
                branch_body,
                vec![core::CompiledRepeatDefinition::new(
                    "ids".to_owned(),
                    vec![param("id", core::CoreType::Int64, false)],
                )],
            )],
        )],
        Vec::new(),
    );
    let query = compiled_query("findUsers", "SELECT id FROM users WHERE 1 = 1;")
        .with_dynamic_body(dynamic_body);

    assert_eq!(
        render_query(&query),
        r#"export type findUsers_Input = {
  filter?: {
    $fragment: "byIds";
    ids: readonly [{ id: string }, ...{ id: string }[]];
  };
};

export type findUsers_Row = {
  id: number;
};

export type findUsers_Output = findUsers_Row[];

export function findUsers(
  input: findUsers_Input,
): { sql: string; params: readonly SqlParam[] } {
  const sqlParts: string[] = [];
  const params: SqlParam[] = [];

  sqlParts.push("SELECT id FROM users WHERE 1 = 1");
  switch (input.filter?.$fragment) {
    case "byIds":
      if (input.filter.ids.length === 0) {
        throw new Error("Repeat `ids` requires at least one item");
      }
      sqlParts.push(" AND id IN (");
      {
        let idsIndex = 0;
        for (const idsItem of input.filter.ids) {
          if (idsIndex > 0) {
            sqlParts.push(",");
          }
          sqlParts.push("?");
          params.push(idsItem.id);
          idsIndex += 1;
        }
      }
      sqlParts.push(")");
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
fn renders_dynamic_query_input_fields_in_source_order() {
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![
            core::CompiledSqlBody::new(
                vec![
                    sql_segment(
                        "SELECT id FROM users WHERE tenant_id = ? AND id IN (",
                        vec![param("tenantId", core::CoreType::String, false)],
                    ),
                    sql_segment(")", Vec::new()),
                ],
                vec![core::CompiledRepeatOccurrence::new(
                    "ids".to_owned(),
                    ",".to_owned(),
                    sql_segment("?", vec![param("id", core::CoreType::Int64, false)]),
                )],
            ),
            core::CompiledSqlBody::from_segment(sql_segment(
                " AND status = ?;",
                vec![param("status", core::CoreType::String, false)],
            )),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![core::CompiledSlotBranch::new(
                "activeOnly".to_owned(),
                vec![sql_segment(" AND active = 1", Vec::new())],
            )],
        )],
        vec![core::CompiledRepeatDefinition::new(
            "ids".to_owned(),
            vec![param("id", core::CoreType::Int64, false)],
        )],
    );
    let query = core::CompiledQuery::new(
        core::QueryId::new("findUsers".to_owned()),
        "SELECT id FROM users WHERE tenant_id = ?;".to_owned(),
        core::Cardinality::Many,
        vec![
            core::InputField::new("tenantId".to_owned(), core::CoreType::String, false),
            core::InputField::new("status".to_owned(), core::CoreType::String, false),
        ],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int32,
            false,
        )],
    )
    .with_params(vec![
        param("tenantId", core::CoreType::String, false),
        param("status", core::CoreType::String, false),
    ])
    .with_dynamic_body(dynamic_body);

    assert!(render_query(&query).starts_with(
        r#"export type findUsers_Input = {
  tenantId: string;
  ids: readonly [{ id: string }, ...{ id: string }[]];
  filter?: { $fragment: "activeOnly" };
  status: string;
};
"#
    ));
}

#[test]
fn renders_slot_branch_input_fields_in_source_order() {
    let branch_body = core::CompiledSqlBody::new(
        vec![
            sql_segment(" AND id IN (", Vec::new()),
            sql_segment(
                ") AND role = ?",
                vec![param("role", core::CoreType::String, false)],
            ),
        ],
        vec![core::CompiledRepeatOccurrence::new(
            "ids".to_owned(),
            ",".to_owned(),
            sql_segment("?", vec![param("id", core::CoreType::Int64, false)]),
        )],
    );
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![
            core::CompiledSqlBody::from_segment(sql_segment(
                "SELECT id FROM users WHERE 1 = 1",
                Vec::new(),
            )),
            core::CompiledSqlBody::from_segment(sql_segment(";", Vec::new())),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![core::CompiledSlotBranch::new_with_body(
                "byIdsAndRole".to_owned(),
                branch_body,
                vec![core::CompiledRepeatDefinition::new(
                    "ids".to_owned(),
                    vec![param("id", core::CoreType::Int64, false)],
                )],
            )],
        )],
        Vec::new(),
    );
    let query = compiled_query("findUsers", "SELECT id FROM users WHERE 1 = 1;")
        .with_dynamic_body(dynamic_body);

    assert!(render_query(&query).starts_with(
        r#"export type findUsers_Input = {
  filter?: {
    $fragment: "byIdsAndRole";
    ids: readonly [{ id: string }, ...{ id: string }[]];
    role: string;
  };
};
"#
    ));
}

#[test]
fn generated_file_with_repeat_query_includes_private_sql_param_alias() {
    let dynamic_body = core::CompiledDynamicQuery::new_with_bodies(
        vec![core::CompiledSqlBody::new(
            vec![
                sql_segment("SELECT id FROM users WHERE id IN (", Vec::new()),
                sql_segment(");", Vec::new()),
            ],
            vec![core::CompiledRepeatOccurrence::new(
                "ids".to_owned(),
                ",".to_owned(),
                sql_segment("?", vec![param("id", core::CoreType::Int64, false)]),
            )],
        )],
        Vec::new(),
        Vec::new(),
        vec![core::CompiledRepeatDefinition::new(
            "ids".to_owned(),
            vec![param("id", core::CoreType::Int64, false)],
        )],
    );
    let repeat_query = compiled_query("findUsers", "SELECT id FROM users WHERE id IN (?);")
        .with_dynamic_body(dynamic_body);
    let static_query = compiled_query("listRoles", "SELECT id FROM roles;");

    let contents = render_generated_file_contents(&[repeat_query, static_query]);

    assert!(contents.starts_with(
        "// @generated by sqlay. Do not edit.\n\n\
type SqlParam = unknown;\n\n\
export type findUsers_Input"
    ));
    assert_eq!(contents.matches("type SqlParam = unknown;").count(), 1);
}
