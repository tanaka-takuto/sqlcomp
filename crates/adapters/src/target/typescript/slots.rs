use std::fmt::Write as _;

use sqlay_core as core;

use super::literals::typescript_string_literal;
use super::types::{
    nested_slot_param_access, typescript_param_binding_type, typescript_property_name,
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
        for segment in branch.segments() {
            render_dynamic_sql_segment(output, "      ", segment, |param| {
                nested_slot_param_access(input_name, slot.id(), param.input_name())
            });
        }
        output.push_str("      break;\n");
    }

    output.push_str("  }\n");
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
