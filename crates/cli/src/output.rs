use sqlcomp_app as app;
use sqlcomp_core as core;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfiguredCommandOutcome {
    Check(core::DiagnosticReport),
    Compile(app::CompileOutcome),
}

impl ConfiguredCommandOutcome {
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        match self {
            Self::Check(diagnostics) => diagnostics,
            Self::Compile(outcome) => outcome.diagnostics(),
        }
    }
}

pub fn print_success_summary(outcome: &ConfiguredCommandOutcome) {
    match outcome {
        ConfiguredCommandOutcome::Check(_) => {
            println!("Check passed. No files written.");
        }
        ConfiguredCommandOutcome::Compile(outcome) => {
            print!(
                "Compile succeeded. Generated or updated {} {}.",
                outcome.generated_file_count(),
                pluralize(outcome.generated_file_count(), "file", "files")
            );

            if let Some(removed_file_count) = outcome.stale_file_removal_count() {
                print!(
                    " Removed {} stale generated {}.",
                    removed_file_count,
                    pluralize(removed_file_count, "file", "files")
                );
            }

            println!();
        }
    }
}

const fn pluralize(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
