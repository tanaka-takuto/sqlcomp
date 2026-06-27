use sqlay_core as core;

use super::slot_variants::SlotExpansionContext;

pub(super) fn query_param_placeholder_index(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<usize> {
    usage.placeholder_index().ok_or_else(|| {
        param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` in query `{}` is missing placeholder position metadata",
                usage.id(),
                query.metadata().id()
            ),
        )
    })
}

pub(super) fn mutation_param_placeholder_index(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<usize> {
    usage.placeholder_index().ok_or_else(|| {
        mutation_param_usage_error(
            mutation,
            usage,
            format!(
                "Param `{}` in mutation `{}` is missing placeholder position metadata",
                usage.id(),
                mutation.metadata().id()
            ),
        )
    })
}

pub(super) fn with_slot_variant_context(
    report: core::DiagnosticReport,
    context: Option<&SlotExpansionContext>,
) -> core::DiagnosticReport {
    let Some(context) = context else {
        return report;
    };

    let mut diagnostics = report.into_diagnostics();
    diagnostics.extend(context.diagnostics());
    core::DiagnosticReport::from_diagnostics(diagnostics)
}

pub(super) fn query_error(
    query: &core::RawQuery,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

pub(super) fn mutation_error(
    mutation: &core::RawMutation,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = mutation.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

pub(super) fn slot_usage_error(
    query: &core::RawQuery,
    usage: &core::SlotUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

pub(super) fn mutation_slot_usage_error(
    mutation: &core::RawMutation,
    usage: &core::SlotUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            mutation
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

pub(super) fn param_usage_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

pub(super) fn mutation_param_usage_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            mutation
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    location_error(location, message)
}

pub(super) fn location_error(
    location: core::SourceLocation,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(message).with_location(location))
}
