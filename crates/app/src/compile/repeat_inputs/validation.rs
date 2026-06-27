use std::collections::{HashMap, HashSet};

use sqlay_core as core;

use super::super::diagnostics::location_error;
use super::super::slot_variants::{SlotExpansionSourceKind, SlotSpec};

pub(in crate::compile) fn validate_query_repeat_inputs(
    query: &core::RawQuery,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
) -> core::DiagnosticResult<()> {
    let scope = BuilderScope {
        kind: SlotExpansionSourceKind::Query,
        id: query.metadata().id(),
        location: query.source_location(),
    };
    validate_builder_repeat_inputs(
        &scope,
        query.param_usages(),
        query.repeat_usages(),
        slot_specs,
    )?;
    validate_fragment_repeat_inputs(&scope, slot_specs, fragments_by_id)
}

pub(in crate::compile) fn validate_mutation_repeat_inputs(
    mutation: &core::RawMutation,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
) -> core::DiagnosticResult<()> {
    let scope = BuilderScope {
        kind: SlotExpansionSourceKind::Mutation,
        id: mutation.metadata().id(),
        location: mutation.source_location(),
    };
    validate_builder_repeat_inputs(
        &scope,
        mutation.param_usages(),
        mutation.repeat_usages(),
        slot_specs,
    )?;
    validate_fragment_repeat_inputs(&scope, slot_specs, fragments_by_id)
}

#[derive(Clone, Copy, Debug)]
struct BuilderScope<'a> {
    kind: SlotExpansionSourceKind,
    id: &'a str,
    location: Option<&'a core::SourceLocation>,
}

impl BuilderScope<'_> {
    const fn label(self) -> &'static str {
        match self.kind {
            SlotExpansionSourceKind::Query => "query",
            SlotExpansionSourceKind::Mutation => "mutation",
        }
    }

    const fn direct_param_label(self) -> &'static str {
        match self.kind {
            SlotExpansionSourceKind::Query => "query direct Param",
            SlotExpansionSourceKind::Mutation => "mutation direct Param",
        }
    }

    const fn namespace_label(self) -> &'static str {
        match self.kind {
            SlotExpansionSourceKind::Query => "query direct Param IDs, Slot IDs, and Repeat IDs",
            SlotExpansionSourceKind::Mutation => {
                "mutation direct Param IDs, Slot IDs, and Repeat IDs"
            }
        }
    }
}

fn validate_builder_repeat_inputs(
    scope: &BuilderScope<'_>,
    direct_param_usages: &[core::ParamUsage],
    repeat_usages: &[core::RepeatUsage],
    slot_specs: &[SlotSpec],
) -> core::DiagnosticResult<()> {
    let direct_param_ids = direct_param_usages
        .iter()
        .map(core::ParamUsage::id)
        .collect::<HashSet<_>>();
    let slot_ids = slot_specs
        .iter()
        .map(|slot| slot.id.as_str())
        .collect::<HashSet<_>>();

    for repeat in repeat_usages {
        if direct_param_ids.contains(repeat.id()) {
            return Err(repeat_scope_error(
                &RepeatScope::Builder(*scope),
                repeat,
                format!(
                    "Repeat `{}` in {} `{}` conflicts with {} `{}`; {} share the generated input namespace",
                    repeat.id(),
                    scope.label(),
                    scope.id,
                    scope.direct_param_label(),
                    repeat.id(),
                    scope.namespace_label(),
                ),
            ));
        }

        if slot_ids.contains(repeat.id()) {
            return Err(repeat_scope_error(
                &RepeatScope::Builder(*scope),
                repeat,
                format!(
                    "Repeat `{}` in {} `{}` conflicts with Slot `{}`; {} share the generated input namespace",
                    repeat.id(),
                    scope.label(),
                    scope.id,
                    repeat.id(),
                    scope.namespace_label(),
                ),
            ));
        }
    }

    validate_repeated_repeat_shapes(&RepeatScope::Builder(*scope), repeat_usages)
}

fn validate_fragment_repeat_inputs(
    scope: &BuilderScope<'_>,
    slot_specs: &[SlotSpec],
    fragments_by_id: &HashMap<&str, &core::RawFragment>,
) -> core::DiagnosticResult<()> {
    for slot in slot_specs {
        for target in &slot.targets {
            let Some(fragment) = fragments_by_id.get(target.as_str()).copied() else {
                continue;
            };
            let branch = FragmentBranchScope {
                builder: *scope,
                slot_id: &slot.id,
                fragment,
            };
            validate_fragment_branch_repeat_inputs(&branch)?;
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct FragmentBranchScope<'a> {
    builder: BuilderScope<'a>,
    slot_id: &'a str,
    fragment: &'a core::RawFragment,
}

fn validate_fragment_branch_repeat_inputs(
    branch: &FragmentBranchScope<'_>,
) -> core::DiagnosticResult<()> {
    let direct_param_ids = branch
        .fragment
        .param_usages()
        .iter()
        .map(core::ParamUsage::id)
        .collect::<HashSet<_>>();

    for repeat in branch.fragment.repeat_usages() {
        if direct_param_ids.contains(repeat.id()) {
            return Err(repeat_scope_error(
                &RepeatScope::FragmentBranch(*branch),
                repeat,
                format!(
                    "Repeat `{}` in Fragment `{}` selected by Slot `{}` in {} `{}` conflicts with Fragment direct Param `{}`; Fragment direct Param IDs and Repeat IDs share the selected Slot branch input namespace",
                    repeat.id(),
                    branch.fragment.metadata().id(),
                    branch.slot_id,
                    branch.builder.label(),
                    branch.builder.id,
                    repeat.id(),
                ),
            ));
        }
    }

    validate_repeated_repeat_shapes(
        &RepeatScope::FragmentBranch(*branch),
        branch.fragment.repeat_usages(),
    )
}

#[derive(Clone, Copy, Debug)]
enum RepeatScope<'a> {
    Builder(BuilderScope<'a>),
    FragmentBranch(FragmentBranchScope<'a>),
}

fn validate_repeated_repeat_shapes(
    scope: &RepeatScope<'_>,
    repeat_usages: &[core::RepeatUsage],
) -> core::DiagnosticResult<()> {
    let mut inputs = Vec::<RepeatInput>::new();

    for repeat in repeat_usages {
        let shape = RepeatItemShape::from_usage(scope, repeat)?;
        if let Some(existing) = inputs.iter_mut().find(|input| input.id == repeat.id()) {
            existing.merge_shape(scope, repeat, shape)?;
        } else {
            inputs.push(RepeatInput {
                id: repeat.id().to_owned(),
                shape,
            });
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepeatInput {
    id: String,
    shape: RepeatItemShape,
}

impl RepeatInput {
    fn merge_shape(
        &mut self,
        scope: &RepeatScope<'_>,
        repeat: &core::RepeatUsage,
        later_shape: RepeatItemShape,
    ) -> core::DiagnosticResult<()> {
        if !self.shape.has_same_field_id_set(&later_shape) {
            return Err(repeat_scope_error(
                scope,
                repeat,
                format!(
                    "{}: first occurrence uses fields {} but conflicting occurrence uses {}; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability",
                    repeat_shape_message_prefix(scope, repeat.id()),
                    self.shape.format_field_ids(),
                    later_shape.format_field_ids(),
                ),
            ));
        }

        for later_field in later_shape.fields {
            let first_field = self
                .shape
                .fields
                .iter_mut()
                .find(|field| field.id == later_field.id)
                .expect("matching field id set should contain the later field");
            first_field.merge(scope, repeat, &later_field)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepeatItemShape {
    fields: Vec<RepeatItemField>,
}

impl RepeatItemShape {
    fn from_usage(
        scope: &RepeatScope<'_>,
        repeat: &core::RepeatUsage,
    ) -> core::DiagnosticResult<Self> {
        let mut fields = Vec::<RepeatItemField>::new();

        for param in repeat.item_param_usages() {
            let field = RepeatItemField::from_param(param);
            if let Some(existing) = fields.iter_mut().find(|item| item.id == field.id) {
                existing.merge(scope, repeat, &field)?;
            } else {
                fields.push(field);
            }
        }

        Ok(Self { fields })
    }

    fn has_same_field_id_set(&self, other: &Self) -> bool {
        self.fields.len() == other.fields.len()
            && self
                .fields
                .iter()
                .all(|field| other.fields.iter().any(|other| other.id == field.id))
    }

    fn format_field_ids(&self) -> String {
        format!(
            "[{}]",
            self.fields
                .iter()
                .map(|field| field.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepeatItemField {
    id: String,
    ty: Option<core::CoreType>,
    nullable: bool,
}

impl RepeatItemField {
    fn from_param(param: &core::ParamUsage) -> Self {
        Self {
            id: param.id().to_owned(),
            ty: param.value_type_override(),
            nullable: param.nullable_override(),
        }
    }

    fn merge(
        &mut self,
        scope: &RepeatScope<'_>,
        repeat: &core::RepeatUsage,
        later: &Self,
    ) -> core::DiagnosticResult<()> {
        if let (Some(first_ty), Some(later_ty)) = (self.ty, later.ty)
            && first_ty != later_ty
        {
            return Err(repeat_scope_error(
                scope,
                repeat,
                format!(
                    "{} item Param `{}` type conflict: first occurrence uses {first_ty:?} but conflicting occurrence uses {later_ty:?}; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability",
                    repeat_shape_message_prefix(scope, repeat.id()),
                    self.id,
                ),
            ));
        }
        if self.ty.is_none() {
            self.ty = later.ty;
        }

        if self.nullable != later.nullable {
            return Err(repeat_scope_error(
                scope,
                repeat,
                format!(
                    "{} item Param `{}` nullability conflict: first occurrence is nullable {} but conflicting occurrence is nullable {}; repeated Repeat IDs must use the same item Param ID set with matching valueType and nullability",
                    repeat_shape_message_prefix(scope, repeat.id()),
                    self.id,
                    self.nullable,
                    later.nullable,
                ),
            ));
        }

        Ok(())
    }
}

fn repeat_shape_message_prefix(scope: &RepeatScope<'_>, repeat_id: &str) -> String {
    match scope {
        RepeatScope::Builder(builder) => format!(
            "conflicting Repeat `{repeat_id}` item shape in {} `{}`",
            builder.label(),
            builder.id
        ),
        RepeatScope::FragmentBranch(branch) => format!(
            "conflicting Repeat `{repeat_id}` item shape in Fragment `{}` selected by Slot `{}` in {} `{}`",
            branch.fragment.metadata().id(),
            branch.slot_id,
            branch.builder.label(),
            branch.builder.id
        ),
    }
}

fn repeat_scope_error(
    scope: &RepeatScope<'_>,
    repeat: &core::RepeatUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location = repeat_location(scope, repeat);
    location_error(location, message)
}

fn repeat_location(scope: &RepeatScope<'_>, repeat: &core::RepeatUsage) -> core::SourceLocation {
    if repeat.source_location().range().is_some() || repeat.source_location().path().is_some() {
        return repeat.source_location().clone();
    }

    match scope {
        RepeatScope::Builder(builder) => builder
            .location
            .cloned()
            .unwrap_or_else(core::SourceLocation::unknown),
        RepeatScope::FragmentBranch(branch) => branch
            .fragment
            .source_location()
            .cloned()
            .unwrap_or_else(core::SourceLocation::unknown),
    }
}
