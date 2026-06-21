use std::path::Path;

use sqlcomp_core as core;

pub(super) fn push_missing_field(
    diagnostics: &mut core::DiagnosticReport,
    name: &str,
    location: Option<&core::SourceLocation>,
) {
    push_error(
        diagnostics,
        format!("missing required config field `{name}`"),
        location,
    );
}

pub(super) fn push_error(
    diagnostics: &mut core::DiagnosticReport,
    message: impl Into<String>,
    location: Option<&core::SourceLocation>,
) {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = location {
        diagnostic = diagnostic.with_location(location.clone());
    }
    diagnostics.push(diagnostic);
}

pub(super) fn single_error_report(
    message: impl Into<String>,
    location: Option<core::SourceLocation>,
) -> core::DiagnosticReport {
    let diagnostic = if let Some(location) = location {
        core::Diagnostic::error(message).with_location(location)
    } else {
        core::Diagnostic::error(message)
    };

    core::DiagnosticReport::new(diagnostic)
}

pub(super) fn parse_error_location(
    path: Option<&Path>,
    error: &serde_json::Error,
) -> Option<core::SourceLocation> {
    let position = core::SourcePosition::one_based(error.line(), error.column())?;

    Some(path.map_or_else(
        || core::SourceLocation::from_range(core::SourceRange::point(position)),
        |path| core::SourceLocation::at_position(path, position),
    ))
}
