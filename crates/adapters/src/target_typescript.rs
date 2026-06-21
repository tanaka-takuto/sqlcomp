//! TypeScript target generation adapter.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use sqlcomp_app::TargetGenerator;
use sqlcomp_core as core;

/// TypeScript symbols generated from one compiled query ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySymbols {
    function: String,
    input_type: String,
    row_type: String,
    output_type: String,
}

impl QuerySymbols {
    /// Build TypeScript symbol names for a compiled query.
    #[must_use]
    pub fn for_query(query: &core::CompiledQuery) -> Self {
        Self::from_query_id(query.id().as_str())
    }

    /// Build TypeScript symbol names from a validated query ID.
    #[must_use]
    pub fn from_query_id(query_id: &str) -> Self {
        Self {
            function: query_id.to_owned(),
            input_type: format!("{query_id}_Input"),
            row_type: format!("{query_id}_Row"),
            output_type: format!("{query_id}_Output"),
        }
    }

    /// Generated query builder function name.
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function
    }

    /// Generated input type alias name.
    #[must_use]
    pub fn input_type_name(&self) -> &str {
        &self.input_type
    }

    /// Generated result row type alias name.
    #[must_use]
    pub fn row_type_name(&self) -> &str {
        &self.row_type
    }

    /// Generated output type alias name.
    #[must_use]
    pub fn output_type_name(&self) -> &str {
        &self.output_type
    }
}

/// Render text as a TypeScript double-quoted string literal.
#[must_use]
pub fn typescript_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');

    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\u{0008}' => literal.push_str("\\b"),
            '\u{000c}' => literal.push_str("\\f"),
            '\u{2028}' => literal.push_str("\\u2028"),
            '\u{2029}' => literal.push_str("\\u2029"),
            control if control.is_control() => {
                let code_point = u32::from(control);
                write!(&mut literal, "\\u{code_point:04X}").expect("writing to String cannot fail");
            }
            other => literal.push(other),
        }
    }

    literal.push('"');
    literal
}

/// Render a generated query builder `sql` property.
#[must_use]
pub fn render_sql_property(query: &core::CompiledQuery) -> String {
    format!("    sql: {},", typescript_string_literal(query.sql()))
}

/// Render a full generated TypeScript file from compiled queries.
#[must_use]
pub fn render_generated_file_contents(queries: &[core::CompiledQuery]) -> String {
    render_generated_file_contents_from_iter(queries.iter())
}

fn render_generated_file_contents_from_iter<'a>(
    queries: impl IntoIterator<Item = &'a core::CompiledQuery>,
) -> String {
    let queries = queries.into_iter().collect::<Vec<_>>();
    let mut contents = String::from(core::GENERATED_FILE_HEADER);
    contents.push_str("\n\n");

    if queries.iter().any(|query| is_slot_query(query)) {
        contents.push_str("type SqlParam = unknown;\n\n");
    }

    let mut is_first_query = true;

    for query in queries {
        if is_first_query {
            is_first_query = false;
        } else {
            contents.push('\n');
        }
        contents.push_str(&render_query(query));
    }

    contents
}

/// Render TypeScript declarations and the SQL builder for one compiled query.
#[must_use]
pub fn render_query(query: &core::CompiledQuery) -> String {
    let symbols = QuerySymbols::for_query(query);
    let mut output = String::new();

    render_input_type_alias(&mut output, query, &symbols);
    output.push('\n');

    writeln!(&mut output, "export type {} = {{", symbols.row_type_name())
        .expect("writing to String cannot fail");
    for column in query.row() {
        writeln!(
            &mut output,
            "  {}: {};",
            typescript_property_name(column.name()),
            typescript_result_type(column)
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("};\n\n");

    writeln!(
        &mut output,
        "export type {} = {};",
        symbols.output_type_name(),
        typescript_output_type(&symbols, query.cardinality())
    )
    .expect("writing to String cannot fail");
    output.push('\n');

    writeln!(&mut output, "export function {}(", symbols.function_name())
        .expect("writing to String cannot fail");
    render_function_input_parameter(&mut output, query, &symbols);
    writeln!(
        &mut output,
        "): {{ sql: string; params: {} }} {{",
        typescript_params_type(query)
    )
    .expect("writing to String cannot fail");
    if let Some(dynamic_body) = query.dynamic_body() {
        render_dynamic_builder_body(&mut output, query, dynamic_body);
    } else {
        render_static_builder_body(&mut output, query);
    }
    output.push_str("}\n");

    output
}

fn render_static_builder_body(output: &mut String, query: &core::CompiledQuery) {
    output.push_str("  return {\n");
    writeln!(output, "{}", render_sql_property(query)).expect("writing to String cannot fail");
    writeln!(
        output,
        "    params: {} as const,",
        typescript_params_expression(query.params())
    )
    .expect("writing to String cannot fail");
    output.push_str("  };\n");
}

fn render_dynamic_builder_body(
    output: &mut String,
    query: &core::CompiledQuery,
    dynamic_body: &core::CompiledDynamicQuery,
) {
    let input_name = function_input_name(query);

    output.push_str("  const sqlParts: string[] = [];\n");
    output.push_str("  const params: SqlParam[] = [];\n\n");

    for (index, segment) in dynamic_body.base_segments().iter().enumerate() {
        render_dynamic_sql_segment(output, "  ", segment, |param| {
            input_param_access(input_name, param.input_name())
        });

        if let Some(occurrence) = dynamic_body.slot_occurrences().get(index) {
            let slot = dynamic_body
                .slots()
                .iter()
                .find(|slot| slot.id() == occurrence.slot_id())
                .expect("compiled dynamic query Slot occurrence must have a Slot definition");
            render_slot_switch(output, input_name, slot);
        }
    }

    output.push('\n');
    output.push_str("  return {\n");
    output.push_str("    sql: sqlParts.join(\"\"),\n");
    output.push_str("    params,\n");
    output.push_str("  };\n");
}

fn render_slot_switch(output: &mut String, input_name: &str, slot: &core::CompiledSlotDefinition) {
    writeln!(
        output,
        "  switch ({}?.$fragment) {{",
        input_param_access(input_name, slot.id())
    )
    .expect("writing to String cannot fail");

    for branch in slot.branches() {
        writeln!(
            output,
            "    case {}:",
            typescript_string_literal(branch.target_id())
        )
        .expect("writing to String cannot fail");
        for segment in branch.segments() {
            render_dynamic_sql_segment(output, "      ", segment, |param| {
                nested_slot_param_access(input_name, slot.id(), param.input_name())
            });
        }
        output.push_str("      break;\n");
    }

    output.push_str("  }\n");
}

fn render_dynamic_sql_segment<F>(
    output: &mut String,
    indent: &str,
    segment: &core::CompiledSqlSegment,
    param_access: F,
) where
    F: Fn(&core::ParamBinding) -> String,
{
    if !segment.sql().is_empty() {
        writeln!(
            output,
            "{indent}sqlParts.push({});",
            typescript_string_literal(segment.sql())
        )
        .expect("writing to String cannot fail");
    }

    for param in segment.params() {
        writeln!(output, "{indent}params.push({});", param_access(param))
            .expect("writing to String cannot fail");
    }
}

fn render_input_type_alias(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    if query.input().is_empty() && !has_slot_inputs(query) {
        writeln!(
            output,
            "export type {} = Record<string, never>;",
            symbols.input_type_name()
        )
        .expect("writing to String cannot fail");
        return;
    }

    writeln!(output, "export type {} = {{", symbols.input_type_name())
        .expect("writing to String cannot fail");
    for field in query.input() {
        writeln!(
            output,
            "  {}: {};",
            typescript_property_name(field.name()),
            typescript_input_field_type(field)
        )
        .expect("writing to String cannot fail");
    }
    if let Some(dynamic_body) = query.dynamic_body() {
        for slot in dynamic_body.slots() {
            render_slot_input_field(output, slot);
        }
    }
    output.push_str("};\n");
}

fn has_slot_inputs(query: &core::CompiledQuery) -> bool {
    query
        .dynamic_body()
        .is_some_and(|dynamic_body| !dynamic_body.slots().is_empty())
}

const fn is_slot_query(query: &core::CompiledQuery) -> bool {
    query.dynamic_body().is_some()
}

fn render_slot_input_field(output: &mut String, slot: &core::CompiledSlotDefinition) {
    let branch_types = slot
        .branches()
        .iter()
        .map(render_slot_branch_input_type)
        .collect::<Vec<_>>();
    let slot_type = if branch_types.is_empty() {
        "never".to_owned()
    } else {
        branch_types.join(" | ")
    };

    writeln!(
        output,
        "  {}?: {};",
        typescript_property_name(slot.id()),
        slot_type
    )
    .expect("writing to String cannot fail");
}

fn render_slot_branch_input_type(branch: &core::CompiledSlotBranch) -> String {
    let params = unique_branch_params(branch);
    let fragment = typescript_string_literal(branch.target_id());

    if params.is_empty() {
        return format!("{{ $fragment: {fragment} }}");
    }

    let mut output = String::new();
    writeln!(&mut output, "{{").expect("writing to String cannot fail");
    writeln!(&mut output, "    $fragment: {fragment};").expect("writing to String cannot fail");
    for param in params {
        writeln!(
            &mut output,
            "    {}: {};",
            typescript_property_name(param.input_name()),
            typescript_param_binding_type(param)
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("  }");
    output
}

fn unique_branch_params(branch: &core::CompiledSlotBranch) -> Vec<&core::ParamBinding> {
    let mut params = Vec::new();

    for segment in branch.segments() {
        for param in segment.params() {
            if !params
                .iter()
                .any(|existing: &&core::ParamBinding| existing.input_name() == param.input_name())
            {
                params.push(param);
            }
        }
    }

    params
}

fn render_function_input_parameter(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    if query.input().is_empty() {
        writeln!(output, "  _input: {} = {{}},", symbols.input_type_name())
            .expect("writing to String cannot fail");
    } else {
        writeln!(output, "  input: {},", symbols.input_type_name())
            .expect("writing to String cannot fail");
    }
}

fn function_input_name(query: &core::CompiledQuery) -> &'static str {
    if query.input().is_empty() {
        "_input"
    } else {
        "input"
    }
}

fn typescript_output_type(symbols: &QuerySymbols, cardinality: core::Cardinality) -> String {
    let row_type = symbols.row_type_name();

    match cardinality {
        core::Cardinality::One => format!("{row_type} | null"),
        core::Cardinality::Many => format!("{row_type}[]"),
    }
}

fn typescript_input_field_type(field: &core::InputField) -> String {
    typescript_nullable_type(field.ty(), field.is_nullable())
}

fn typescript_param_binding_type(param: &core::ParamBinding) -> String {
    typescript_nullable_type(param.ty(), param.is_nullable())
}

fn typescript_result_type(column: &core::ResultColumn) -> String {
    typescript_nullable_type(column.ty(), column.is_nullable())
}

fn typescript_nullable_type(ty: core::CoreType, nullable: bool) -> String {
    let base_type = typescript_core_type(ty);

    if nullable {
        format!("{base_type} | null")
    } else {
        base_type.to_owned()
    }
}

const fn typescript_core_type(ty: core::CoreType) -> &'static str {
    match ty {
        core::CoreType::Bool => "boolean",
        core::CoreType::Int32 | core::CoreType::Float64 => "number",
        core::CoreType::Int64
        | core::CoreType::Decimal
        | core::CoreType::Date
        | core::CoreType::Time
        | core::CoreType::DateTime
        | core::CoreType::String => "string",
        core::CoreType::Bytes => "Uint8Array",
        core::CoreType::Json | core::CoreType::Unknown => "unknown",
    }
}

fn typescript_property_name(name: &str) -> String {
    if is_simple_typescript_identifier(name) {
        name.to_owned()
    } else {
        typescript_string_literal(name)
    }
}

fn typescript_params_tuple_type(params: &[core::ParamBinding]) -> String {
    if params.is_empty() {
        "readonly []".to_owned()
    } else {
        format!(
            "readonly [{}]",
            params
                .iter()
                .map(typescript_param_binding_type)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn typescript_params_type(query: &core::CompiledQuery) -> String {
    if is_slot_query(query) {
        "readonly SqlParam[]".to_owned()
    } else {
        typescript_params_tuple_type(query.params())
    }
}

fn typescript_params_expression(params: &[core::ParamBinding]) -> String {
    if params.is_empty() {
        "[]".to_owned()
    } else {
        format!(
            "[{}]",
            params
                .iter()
                .map(|param| format!("input.{}", param.input_name()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn input_param_access(input_name: &str, param_name: &str) -> String {
    typescript_property_access(input_name, param_name)
}

fn nested_slot_param_access(input_name: &str, slot_id: &str, param_name: &str) -> String {
    typescript_property_access(&typescript_property_access(input_name, slot_id), param_name)
}

fn typescript_property_access(base: &str, property: &str) -> String {
    if is_simple_typescript_identifier(property) {
        format!("{base}.{property}")
    } else {
        format!("{base}[{}]", typescript_string_literal(property))
    }
}

fn is_simple_typescript_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    is_identifier_start(first) && chars.all(is_identifier_continue)
}

const fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

const fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

/// Dummy TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(
        &self,
        plan: &core::CompilationPlan,
        queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        let mut queries_by_source_path: BTreeMap<PathBuf, Vec<&core::CompiledQuery>> =
            BTreeMap::new();

        for query in queries {
            let source_path = query_source_path(query)?;
            queries_by_source_path
                .entry(source_path.to_path_buf())
                .or_default()
                .push(query);
        }

        let mut files = Vec::with_capacity(queries_by_source_path.len());
        for (source_path, source_queries) in queries_by_source_path {
            let output_path = generated_typescript_path(plan.output_dir(), &source_path);
            let contents = render_generated_file_contents_from_iter(source_queries);
            files.push(core::GeneratedFile::new(output_path, contents));
        }

        Ok(core::GeneratedFiles::new(files))
    }
}

fn query_source_path(query: &core::CompiledQuery) -> core::DiagnosticResult<&Path> {
    let Some(source_path) = query.source_path() else {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled query `{}` does not include a source file path for output mapping",
                query.id().as_str()
            ),
        )));
    };

    if !is_safe_relative_path(source_path) {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled query `{}` has invalid source file path `{}`; expected a config-relative SQL path",
                query.id().as_str(),
                source_path.display()
            ),
        )));
    }

    Ok(source_path)
}

fn generated_typescript_path(output_dir: &Path, source_relative_path: &Path) -> PathBuf {
    output_dir.join(source_relative_path).with_extension("ts")
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.file_name().is_some()
        && path
            .components()
            .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        QuerySymbols, TypeScriptTargetGenerator, render_generated_file_contents, render_query,
        render_sql_property, typescript_string_literal,
    };
    use sqlcomp_app::TargetGenerator;
    use sqlcomp_core as core;

    #[test]
    fn query_symbols_use_id_exactly_with_fixed_suffixes() {
        for query_id in ["listUsers", "list_users", "_findUser2", "HTTPStatus200"] {
            let symbols = QuerySymbols::from_query_id(query_id);

            assert_eq!(symbols.function_name(), query_id);
            assert_eq!(symbols.input_type_name(), format!("{query_id}_Input"));
            assert_eq!(symbols.row_type_name(), format!("{query_id}_Row"));
            assert_eq!(symbols.output_type_name(), format!("{query_id}_Output"));
        }
    }

    #[test]
    fn query_symbols_are_derived_from_compiled_query_id_without_transformation() {
        let query = core::CompiledQuery::new(
            core::QueryId::new("list_users".to_owned()),
            "SELECT id FROM users;".to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            Vec::new(),
        );

        let symbols = QuerySymbols::for_query(&query);

        assert_eq!(symbols.function_name(), "list_users");
        assert_eq!(symbols.input_type_name(), "list_users_Input");
        assert_eq!(symbols.row_type_name(), "list_users_Row");
        assert_eq!(symbols.output_type_name(), "list_users_Output");
    }

    #[test]
    fn sql_literal_uses_double_quotes_for_template_literal_hazards() {
        let sql = "SELECT `id`, '${literal}' FROM `users` WHERE note = '${not_param}';";

        assert_eq!(
            typescript_string_literal(sql),
            r#""SELECT `id`, '${literal}' FROM `users` WHERE note = '${not_param}';""#
        );
    }

    #[test]
    fn sql_literal_escapes_quotes_backslashes_and_line_breaks() {
        let sql = "SELECT \"quoted\", 'single', C:\\tmp\\users\nFROM users\r\nWHERE tab = '\t';";

        assert_eq!(
            typescript_string_literal(sql),
            r#""SELECT \"quoted\", 'single', C:\\tmp\\users\nFROM users\r\nWHERE tab = '\t';""#
        );
    }

    #[test]
    fn sql_literal_escapes_javascript_line_separators_and_other_controls() {
        let sql = "SELECT '\u{0001}\u{2028}\u{2029}';";

        assert_eq!(
            typescript_string_literal(sql),
            r#""SELECT '\u0001\u2028\u2029';""#
        );
    }

    #[test]
    fn rendered_sql_property_uses_safe_literal() {
        let query = core::CompiledQuery::new(
            core::QueryId::new("findNotes".to_owned()),
            "SELECT `body`, '${not_param}'\nFROM notes;".to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(
            render_sql_property(&query),
            r#"    sql: "SELECT `body`, '${not_param}'\nFROM notes;","#
        );
    }

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

    #[test]
    fn generator_keeps_slotless_files_on_static_builder_surface_when_slots_are_compiled_elsewhere()
    {
        let plan = compilation_plan();
        let no_param_query =
            compiled_query("listUsers", "SELECT id FROM users;").with_source_path("sql/static.sql");
        let param_query = core::CompiledQuery::new(
            core::QueryId::new("findUserByEmail".to_owned()),
            "SELECT id FROM users WHERE email = ?;".to_owned(),
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
        .with_params(vec![param("email", core::CoreType::String, false)])
        .with_source_path("sql/static.sql");
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
        let slot_query = compiled_query("searchUsers", "SELECT id FROM users WHERE 1 = 1;")
            .with_dynamic_body(dynamic_body)
            .with_source_path("sql/dynamic.sql");

        let files = TypeScriptTargetGenerator
            .generate(&plan, &[no_param_query, param_query, slot_query])
            .expect("generator should preserve each file's generated surface independently");

        let static_contents = file_contents(
            &files,
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/static.ts"),
        );
        let dynamic_contents = file_contents(
            &files,
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/dynamic.ts"),
        );

        assert!(!static_contents.contains("type SqlParam = unknown;"));
        assert!(!static_contents.contains("sqlParts"));
        assert!(!static_contents.contains("readonly SqlParam[]"));
        assert!(static_contents.contains("export type listUsers_Input = Record<string, never>;"));
        assert!(static_contents.contains(
            "export function listUsers(\n  _input: listUsers_Input = {},\n): { sql: string; params: readonly [] }"
        ));
        assert!(static_contents.contains(r#"sql: "SELECT id FROM users;","#));
        assert!(static_contents.contains("params: [] as const,"));
        assert!(static_contents.contains(
            "export function findUserByEmail(\n  input: findUserByEmail_Input,\n): { sql: string; params: readonly [string] }"
        ));
        assert!(static_contents.contains(r#"sql: "SELECT id FROM users WHERE email = ?;","#));
        assert!(static_contents.contains("params: [input.email] as const,"));

        assert!(dynamic_contents.contains("type SqlParam = unknown;"));
        assert!(dynamic_contents.contains("sqlParts.join(\"\")"));
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
        assert!(
            contents.contains("export type findLatestUser_Output = findLatestUser_Row | null;")
        );
    }

    #[test]
    fn generator_maps_nested_sql_paths_under_output_dir() {
        let plan = compilation_plan();
        let query = compiled_query("listAdmins", "SELECT id FROM admins;")
            .with_source_path("sql/admin/users.sql");

        let files = TypeScriptTargetGenerator
            .generate(&plan, &[query])
            .expect("generator should map SQL source path to TypeScript output path");

        assert_eq!(files.files().len(), 1);
        assert_eq!(
            files.files()[0].path(),
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/admin/users.ts")
        );
        assert!(
            files.files()[0]
                .contents()
                .contains("export function listAdmins(")
        );
    }

    #[test]
    fn generator_generates_param_queries() {
        let plan = compilation_plan();
        let query = core::CompiledQuery::new(
            core::QueryId::new("findUser".to_owned()),
            "SELECT id FROM users WHERE email = ?;".to_owned(),
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
        )])
        .with_source_path("sql/users.sql");

        let files = TypeScriptTargetGenerator
            .generate(&plan, &[query])
            .expect("Param TypeScript generation should emit input and params");

        let users_contents = file_contents(
            &files,
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts"),
        );
        assert!(users_contents.contains("export type findUser_Input = {\n  email: string;\n};"));
        assert!(users_contents.contains(
            "export function findUser(\n  input: findUser_Input,\n): { sql: string; params: readonly [string] }"
        ));
        assert!(users_contents.contains("params: [input.email] as const"));
    }

    #[test]
    fn generator_combines_queries_from_same_sql_file_into_one_module() {
        let plan = compilation_plan();
        let queries = [
            compiled_query("listUsers", "SELECT id FROM users;").with_source_path("sql/users.sql"),
            compiled_query("findLatestUser", "SELECT id FROM users LIMIT 1;")
                .with_source_path("sql/users.sql"),
            compiled_query("listRoles", "SELECT id FROM roles;")
                .with_source_path("sql/admin/roles.sql"),
        ];

        let files = TypeScriptTargetGenerator
            .generate(&plan, &queries)
            .expect("generator should group queries by source SQL file");

        assert_eq!(files.files().len(), 2);
        let users_contents = file_contents(
            &files,
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts"),
        );
        assert!(users_contents.contains("export function listUsers("));
        assert!(users_contents.contains("export function findLatestUser("));
        assert!(!users_contents.contains("export function listRoles("));
    }

    fn compilation_plan() -> core::CompilationPlan {
        core::CompilationPlan::new(
            PathBuf::from("/tmp/sqlcomp-project"),
            vec![PathBuf::from("/tmp/sqlcomp-project/sql/**/*.sql")],
            Vec::new(),
            PathBuf::from("/tmp/sqlcomp-project/src/generated/sqlcomp"),
            core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            core::TargetConfig::new(core::TargetLanguage::TypeScript),
        )
    }

    fn compiled_query(id: &str, sql: &str) -> core::CompiledQuery {
        core::CompiledQuery::new(
            core::QueryId::new(id.to_owned()),
            sql.to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            vec![core::ResultColumn::new(
                "id".to_owned(),
                core::CoreType::Int32,
                false,
            )],
        )
    }

    fn slot_definition(
        id: &str,
        branches: Vec<core::CompiledSlotBranch>,
    ) -> core::CompiledSlotDefinition {
        core::CompiledSlotDefinition::new(id.to_owned(), branches)
    }

    fn slot_branch(
        target_id: &str,
        sql: &str,
        params: Vec<core::ParamBinding>,
    ) -> core::CompiledSlotBranch {
        core::CompiledSlotBranch::new(target_id.to_owned(), vec![sql_segment(sql, params)])
    }

    fn sql_segment(sql: &str, params: Vec<core::ParamBinding>) -> core::CompiledSqlSegment {
        core::CompiledSqlSegment::new(sql.to_owned(), params)
    }

    fn param(name: &str, ty: core::CoreType, nullable: bool) -> core::ParamBinding {
        core::ParamBinding::new(name.to_owned(), ty, nullable)
    }

    fn file_contents<'a>(files: &'a core::GeneratedFiles, path: &Path) -> &'a str {
        files
            .files()
            .iter()
            .find(|file| file.path() == path)
            .unwrap_or_else(|| panic!("expected generated file `{}`", path.display()))
            .contents()
    }
}
