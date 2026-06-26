use sqlay_core as core;

pub fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    source_unit_error(query.source_location(), message)
}

pub fn mutation_error(
    mutation: &core::RawMutation,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    source_unit_error(mutation.source_location(), message)
}

pub fn param_usage_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message)
            .with_location(param_location(query.source_location(), usage)),
    )
}

pub fn mutation_param_usage_error(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message)
            .with_location(param_location(mutation.source_location(), usage)),
    )
}

fn source_unit_error(
    source_location: Option<&core::SourceLocation>,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = source_location {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

fn param_location(
    source_location: Option<&core::SourceLocation>,
    usage: &core::ParamUsage,
) -> core::SourceLocation {
    if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
        usage.source_location().clone()
    } else {
        source_location
            .cloned()
            .unwrap_or_else(core::SourceLocation::unknown)
    }
}
