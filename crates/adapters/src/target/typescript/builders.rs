use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::slots::{is_slot_query, render_dynamic_sql_segment, render_slot_switch};
use super::symbols::QuerySymbols;
use super::types::{
    function_input_name, input_param_access, render_function_input_parameter,
    render_input_type_alias, typescript_output_type, typescript_params_expression,
    typescript_params_type, typescript_property_name, typescript_result_type,
};

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

pub(super) fn render_generated_file_contents_from_iter<'a>(
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
