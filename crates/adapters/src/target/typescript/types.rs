use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::slots::render_slot_input_field;
use super::symbols::QuerySymbols;

pub(super) fn render_input_type_alias(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    render_dynamic_input_type_alias(
        output,
        symbols.input_type_name(),
        query.input(),
        query.dynamic_body(),
    );
}

pub(super) fn render_dynamic_input_type_alias(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
) {
    let has_slot_inputs = dynamic_body.is_some_and(|body| !body.slots().is_empty());
    if !has_slot_inputs {
        render_static_input_type_alias(output, input_type_name, input);
        return;
    }

    writeln!(output, "export type {input_type_name} = {{").expect("writing to String cannot fail");
    for field in input {
        writeln!(
            output,
            "  {}: {};",
            typescript_property_name(field.name()),
            typescript_input_field_type(field)
        )
        .expect("writing to String cannot fail");
    }
    if let Some(dynamic_body) = dynamic_body {
        for slot in dynamic_body.slots() {
            render_slot_input_field(output, slot);
        }
    }
    output.push_str("};\n");
}

pub(super) fn render_static_input_type_alias(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
) {
    if input.is_empty() {
        writeln!(
            output,
            "export type {input_type_name} = Record<string, never>;"
        )
        .expect("writing to String cannot fail");
        return;
    }

    writeln!(output, "export type {input_type_name} = {{").expect("writing to String cannot fail");
    for field in input {
        writeln!(
            output,
            "  {}: {};",
            typescript_property_name(field.name()),
            typescript_input_field_type(field)
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("};\n");
}

pub(super) fn render_function_input_parameter(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    render_static_function_input_parameter(output, symbols.input_type_name(), query.input());
}

pub(super) fn render_static_function_input_parameter(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
) {
    let (input_name, default) = if input.is_empty() {
        ("_input", " = {}")
    } else {
        ("input", "")
    };
    writeln!(output, "  {input_name}: {input_type_name}{default},")
        .expect("writing to String cannot fail");
}

pub(super) fn function_input_name(query: &core::CompiledQuery) -> &'static str {
    function_input_name_for_input(query.input())
}

pub(super) const fn function_input_name_for_input(input: &[core::InputField]) -> &'static str {
    if input.is_empty() { "_input" } else { "input" }
}

pub(super) fn typescript_output_type(
    symbols: &QuerySymbols,
    cardinality: core::Cardinality,
) -> String {
    let row_type = symbols.row_type_name();

    match cardinality {
        core::Cardinality::One => format!("{row_type} | null"),
        core::Cardinality::Many => format!("{row_type}[]"),
    }
}

fn typescript_input_field_type(field: &core::InputField) -> String {
    typescript_nullable_type(field.ty(), field.is_nullable())
}

pub(super) fn typescript_param_binding_type(param: &core::ParamBinding) -> String {
    typescript_nullable_type(param.ty(), param.is_nullable())
}

pub(super) fn typescript_result_type(column: &core::ResultColumn) -> String {
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

pub(super) fn typescript_property_name(name: &str) -> String {
    if is_simple_typescript_identifier(name) {
        name.to_owned()
    } else {
        typescript_string_literal(name)
    }
}

pub(super) fn typescript_params_tuple_type(params: &[core::ParamBinding]) -> String {
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

pub(super) fn typescript_params_type(query: &core::CompiledQuery) -> String {
    typescript_dynamic_params_type(query.dynamic_body(), query.params())
}

pub(super) fn typescript_dynamic_params_type(
    dynamic_body: Option<&core::CompiledDynamicQuery>,
    params: &[core::ParamBinding],
) -> String {
    if dynamic_body.is_some() {
        "readonly SqlParam[]".to_owned()
    } else {
        typescript_params_tuple_type(params)
    }
}

pub(super) fn typescript_params_expression(params: &[core::ParamBinding]) -> String {
    if params.is_empty() {
        "[]".to_owned()
    } else {
        format!(
            "[{}]",
            params
                .iter()
                .map(|param| input_param_access("input", param.input_name()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub(super) fn input_param_access(input_name: &str, param_name: &str) -> String {
    typescript_property_access(input_name, param_name)
}

pub(super) fn nested_slot_param_access(
    input_name: &str,
    slot_id: &str,
    param_name: &str,
) -> String {
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
