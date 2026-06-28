use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::types::{
    input_param_access, nested_slot_param_access, render_param_binding_input_field,
    render_repeat_input_field, typescript_property_name,
};

pub(super) const fn is_slot_query(query: &core::CompiledQuery) -> bool {
    query.dynamic_body().is_some()
}

pub(super) const fn is_slot_mutation(mutation: &core::CompiledMutation) -> bool {
    mutation.dynamic_body().is_some()
}

pub(super) fn render_slot_switch(
    output: &mut String,
    input_name: &str,
    slot: &core::CompiledSlotDefinition,
) {
    writeln!(
        output,
        "  switch ({}?.$fragment) {{",
        super::types::input_param_access(input_name, slot.id())
    )
    .expect("writing to String cannot fail");

    for branch in slot.branches() {
        writeln!(
            output,
            "    case {}:",
            typescript_string_literal(branch.target_id())
        )
        .expect("writing to String cannot fail");

        render_repeat_guards(
            output,
            "      ",
            branch.repeats(),
            |repeat_id| nested_slot_param_access(input_name, slot.id(), repeat_id),
            false,
        );

        render_dynamic_sql_body(
            output,
            "      ",
            branch.body(),
            |param| nested_slot_param_access(input_name, slot.id(), param.input_name()),
            |repeat_id| nested_slot_param_access(input_name, slot.id(), repeat_id),
        );
        output.push_str("      break;\n");
    }

    output.push_str("  }\n");
}

pub(super) fn render_dynamic_sql_body<F, R>(
    output: &mut String,
    indent: &str,
    body: &core::CompiledSqlBody,
    param_access: F,
    repeat_input_access: R,
) where
    F: Fn(&core::ParamBinding) -> String,
    R: Fn(&str) -> String,
{
    for (index, segment) in body.base_segments().iter().enumerate() {
        render_dynamic_sql_segment(output, indent, segment, &param_access);

        if let Some(repeat) = body.repeat_occurrences().get(index) {
            render_repeat_occurrence(output, indent, repeat, &repeat_input_access);
        }
    }
}

pub(super) fn render_dynamic_sql_segment<F>(
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

pub(super) fn render_repeat_guards<F>(
    output: &mut String,
    indent: &str,
    repeats: &[core::CompiledRepeatDefinition],
    repeat_input_access: F,
    trailing_blank_line: bool,
) where
    F: Fn(&str) -> String,
{
    for repeat in repeats {
        let input_access = repeat_input_access(repeat.id());
        let error_message = format!("Repeat `{}` requires at least one item", repeat.id());
        writeln!(output, "{indent}if ({input_access}.length === 0) {{")
            .expect("writing to String cannot fail");
        writeln!(
            output,
            "{indent}  throw new Error({});",
            typescript_string_literal(&error_message)
        )
        .expect("writing to String cannot fail");
        writeln!(output, "{indent}}}").expect("writing to String cannot fail");
    }

    if trailing_blank_line && !repeats.is_empty() {
        output.push('\n');
    }
}

fn render_repeat_occurrence<F>(
    output: &mut String,
    indent: &str,
    repeat: &core::CompiledRepeatOccurrence,
    repeat_input_access: &F,
) where
    F: Fn(&str) -> String,
{
    let index_name = format!("{}Index", repeat.repeat_id());
    let item_name = format!("{}Item", repeat.repeat_id());
    let input_access = repeat_input_access(repeat.repeat_id());
    let item_indent = format!("{indent}    ");

    writeln!(output, "{indent}{{").expect("writing to String cannot fail");
    writeln!(output, "{indent}  let {index_name} = 0;").expect("writing to String cannot fail");
    writeln!(
        output,
        "{indent}  for (const {item_name} of {input_access}) {{"
    )
    .expect("writing to String cannot fail");
    writeln!(output, "{indent}    if ({index_name} > 0) {{")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "{indent}      sqlParts.push({});",
        typescript_string_literal(repeat.separator())
    )
    .expect("writing to String cannot fail");
    writeln!(output, "{indent}    }}").expect("writing to String cannot fail");
    render_dynamic_sql_segment(output, &item_indent, repeat.item_segment(), |param| {
        input_param_access(&item_name, param.input_name())
    });
    writeln!(output, "{item_indent}{index_name} += 1;").expect("writing to String cannot fail");
    writeln!(output, "{indent}  }}").expect("writing to String cannot fail");
    writeln!(output, "{indent}}}").expect("writing to String cannot fail");
}

pub(super) fn render_slot_input_field(output: &mut String, slot: &core::CompiledSlotDefinition) {
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
    let fragment = typescript_string_literal(branch.target_id());

    let mut fields = String::new();
    render_slot_branch_input_fields(&mut fields, branch);

    if fields.is_empty() {
        return format!("{{ $fragment: {fragment} }}");
    }

    let mut output = String::new();
    writeln!(&mut output, "{{").expect("writing to String cannot fail");
    writeln!(&mut output, "    $fragment: {fragment};").expect("writing to String cannot fail");
    output.push_str(&fields);
    output.push_str("  }");
    output
}

fn render_slot_branch_input_fields(output: &mut String, branch: &core::CompiledSlotBranch) {
    let params = unique_branch_params(branch);
    let mut rendered_params = Vec::new();
    let mut rendered_repeats = Vec::new();

    for (index, segment) in branch.body().base_segments().iter().enumerate() {
        for param in segment.params() {
            render_branch_param_input_field(
                output,
                &params,
                param.input_name(),
                &mut rendered_params,
            );
        }

        if let Some(repeat) = branch.body().repeat_occurrences().get(index) {
            render_branch_repeat_input_field(
                output,
                branch,
                repeat.repeat_id(),
                &mut rendered_repeats,
            );
        }
    }

    for param in &params {
        render_branch_param_input_field(output, &params, param.input_name(), &mut rendered_params);
    }
    for repeat in branch.repeats() {
        render_branch_repeat_input_field(output, branch, repeat.id(), &mut rendered_repeats);
    }
}

fn render_branch_param_input_field(
    output: &mut String,
    params: &[&core::ParamBinding],
    name: &str,
    rendered_params: &mut Vec<String>,
) {
    if rendered_params.iter().any(|rendered| rendered == name) {
        return;
    }

    if let Some(param) = params.iter().find(|param| param.input_name() == name) {
        render_param_binding_input_field(output, "    ", param);
        rendered_params.push(param.input_name().to_owned());
    }
}

fn render_branch_repeat_input_field(
    output: &mut String,
    branch: &core::CompiledSlotBranch,
    id: &str,
    rendered_repeats: &mut Vec<String>,
) {
    if rendered_repeats.iter().any(|rendered| rendered == id) {
        return;
    }

    if let Some(repeat) = branch.repeats().iter().find(|repeat| repeat.id() == id) {
        render_repeat_input_field(output, "    ", repeat);
        rendered_repeats.push(repeat.id().to_owned());
    }
}

fn unique_branch_params(branch: &core::CompiledSlotBranch) -> Vec<&core::ParamBinding> {
    let mut params = Vec::new();

    for segment in branch.body().base_segments() {
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
