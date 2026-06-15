use std::process::ExitCode;

use sqlcomp_core as core;

pub fn fail(report: &core::DiagnosticReport) -> ExitCode {
    eprintln!("{report}");
    ExitCode::FAILURE
}

pub fn print_diagnostics(report: &core::DiagnosticReport) {
    if !report.is_empty() {
        eprintln!("{report}");
    }
}

pub fn single_cli_error(message: impl Into<String>) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(message))
}
