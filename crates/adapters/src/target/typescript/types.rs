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
    let Some(dynamic_body) = dynamic_body else {
        render_static_input_type_alias(output, input_type_name, input);
        return;
    };

    if dynamic_body.slots().is_empty() && dynamic_body.repeats().is_empty() {
        render_static_input_type_alias(output, input_type_name, input);
        return;
    }

    writeln!(output, "export type {input_type_name} = {{").expect("writing to String cannot fail");
    render_dynamic_input_fields(output, input, dynamic_body);
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
        render_input_field(output, "  ", field);
    }
    output.push_str("};\n");
}

pub(super) fn render_function_input_parameter(
    output: &mut String,
    query: &core::CompiledQuery,
    symbols: &QuerySymbols,
) {
    render_dynamic_function_input_parameter(
        output,
        symbols.input_type_name(),
        query.input(),
        query.dynamic_body(),
    );
}

pub(super) fn render_dynamic_function_input_parameter(
    output: &mut String,
    input_type_name: &str,
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
) {
    let (input_name, default) = if input.is_empty() && !dynamic_body_requires_input(dynamic_body) {
        ("_input", " = {}")
    } else {
        ("input", "")
    };
    writeln!(output, "  {input_name}: {input_type_name}{default},")
        .expect("writing to String cannot fail");
}

pub(super) fn function_input_name(query: &core::CompiledQuery) -> &'static str {
    function_input_name_for_dynamic_body(query.input(), query.dynamic_body())
}

pub(super) fn function_input_name_for_dynamic_body(
    input: &[core::InputField],
    dynamic_body: Option<&core::CompiledDynamicQuery>,
) -> &'static str {
    if input.is_empty() && !dynamic_body_requires_input(dynamic_body) {
        "_input"
    } else {
        "input"
    }
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

pub(super) fn render_repeat_input_field(
    output: &mut String,
    indent: &str,
    repeat: &core::CompiledRepeatDefinition,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(repeat.id()),
        typescript_repeat_input_type(repeat)
    )
    .expect("writing to String cannot fail");
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

fn render_input_field(output: &mut String, indent: &str, field: &core::InputField) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(field.name()),
        typescript_input_field_type(field)
    )
    .expect("writing to String cannot fail");
}

fn render_dynamic_input_fields(
    output: &mut String,
    input: &[core::InputField],
    dynamic_body: &core::CompiledDynamicQuery,
) {
    let mut rendered_fields = Vec::new();
    let mut rendered_repeats = Vec::new();
    let mut rendered_slots = Vec::new();

    for (body_index, body) in dynamic_body.base_bodies().iter().enumerate() {
        for (segment_index, segment) in body.base_segments().iter().enumerate() {
            for param in segment.params() {
                render_dynamic_direct_input_field(
                    output,
                    input,
                    param.input_name(),
                    &mut rendered_fields,
                );
            }

            if let Some(repeat) = body.repeat_occurrences().get(segment_index) {
                render_dynamic_repeat_input_field(
                    output,
                    dynamic_body.repeats(),
                    repeat.repeat_id(),
                    &mut rendered_repeats,
                );
            }
        }

        if let Some(slot) = dynamic_body.slot_occurrences().get(body_index) {
            render_dynamic_slot_input_field(
                output,
                dynamic_body.slots(),
                slot.slot_id(),
                &mut rendered_slots,
            );
        }
    }

    for field in input {
        render_dynamic_direct_input_field(output, input, field.name(), &mut rendered_fields);
    }
    for repeat in dynamic_body.repeats() {
        render_dynamic_repeat_input_field(
            output,
            dynamic_body.repeats(),
            repeat.id(),
            &mut rendered_repeats,
        );
    }
    for slot in dynamic_body.slots() {
        render_dynamic_slot_input_field(
            output,
            dynamic_body.slots(),
            slot.id(),
            &mut rendered_slots,
        );
    }
}

fn render_dynamic_direct_input_field(
    output: &mut String,
    input: &[core::InputField],
    name: &str,
    rendered_fields: &mut Vec<String>,
) {
    if rendered_fields.iter().any(|rendered| rendered == name) {
        return;
    }

    if let Some(field) = input.iter().find(|field| field.name() == name) {
        render_input_field(output, "  ", field);
        rendered_fields.push(field.name().to_owned());
    }
}

fn render_dynamic_repeat_input_field(
    output: &mut String,
    repeats: &[core::CompiledRepeatDefinition],
    id: &str,
    rendered_repeats: &mut Vec<String>,
) {
    if rendered_repeats.iter().any(|rendered| rendered == id) {
        return;
    }

    if let Some(repeat) = repeats.iter().find(|repeat| repeat.id() == id) {
        render_repeat_input_field(output, "  ", repeat);
        rendered_repeats.push(repeat.id().to_owned());
    }
}

fn render_dynamic_slot_input_field(
    output: &mut String,
    slots: &[core::CompiledSlotDefinition],
    id: &str,
    rendered_slots: &mut Vec<String>,
) {
    if rendered_slots.iter().any(|rendered| rendered == id) {
        return;
    }

    if let Some(slot) = slots.iter().find(|slot| slot.id() == id) {
        render_slot_input_field(output, slot);
        rendered_slots.push(slot.id().to_owned());
    }
}

pub(super) fn render_param_binding_input_field(
    output: &mut String,
    indent: &str,
    param: &core::ParamBinding,
) {
    writeln!(
        output,
        "{indent}{}: {};",
        typescript_property_name(param.input_name()),
        typescript_param_binding_type(param)
    )
    .expect("writing to String cannot fail");
}

fn typescript_repeat_input_type(repeat: &core::CompiledRepeatDefinition) -> String {
    let item_type = typescript_repeat_item_type(repeat.fields());
    format!("readonly [{item_type}, ...{item_type}[]]")
}

fn typescript_repeat_item_type(fields: &[core::ParamBinding]) -> String {
    let fields = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                typescript_property_name(field.input_name()),
                typescript_param_binding_type(field)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{{ {fields} }}")
}

fn dynamic_body_requires_input(dynamic_body: Option<&core::CompiledDynamicQuery>) -> bool {
    let Some(dynamic_body) = dynamic_body else {
        return false;
    };

    !dynamic_body.repeats().is_empty()
        || dynamic_body.slots().iter().any(|slot| {
            slot.branches()
                .iter()
                .any(|branch| !branch.repeats().is_empty())
        })
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
