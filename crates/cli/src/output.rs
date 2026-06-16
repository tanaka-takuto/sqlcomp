use sqlcomp_app as app;
use sqlcomp_core as core;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfiguredCommandOutcome {
    Check(app::CheckOutcome),
    Compile(app::CompileOutcome),
}

impl ConfiguredCommandOutcome {
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        match self {
            Self::Check(outcome) => outcome.diagnostics(),
            Self::Compile(outcome) => outcome.diagnostics(),
        }
    }
}

pub fn print_success_summary(outcome: &ConfiguredCommandOutcome) {
    print!("{}", format_success_summary(outcome));
}

pub fn format_success_summary(outcome: &ConfiguredCommandOutcome) -> String {
    let mut output = String::new();

    match outcome {
        ConfiguredCommandOutcome::Check(outcome) => {
            writeln!(
                &mut output,
                "Check passed. Matched {} SQL {}. Compiled {} {}. Output dir: {}. No files written.",
                outcome.source_file_count(),
                pluralize(outcome.source_file_count(), "file", "files"),
                outcome.query_count(),
                pluralize(outcome.query_count(), "query", "queries"),
                outcome.output_dir().display()
            )
            .expect("writing to String cannot fail");
            append_query_summaries(&mut output, outcome.query_summaries());
        }
        ConfiguredCommandOutcome::Compile(outcome) => {
            write!(
                &mut output,
                "Compile succeeded. Matched {} SQL {}. Compiled {} {}. Generated or updated {} {}.",
                outcome.source_file_count(),
                pluralize(outcome.source_file_count(), "file", "files"),
                outcome.query_count(),
                pluralize(outcome.query_count(), "query", "queries"),
                outcome.generated_file_count(),
                pluralize(outcome.generated_file_count(), "file", "files")
            )
            .expect("writing to String cannot fail");

            if let Some(removed_file_count) = outcome.stale_file_removal_count() {
                write!(
                    &mut output,
                    " Removed {} stale generated {}.",
                    removed_file_count,
                    pluralize(removed_file_count, "file", "files")
                )
                .expect("writing to String cannot fail");
            }

            writeln!(
                &mut output,
                " Output dir: {}.",
                outcome.output_dir().display()
            )
            .expect("writing to String cannot fail");
            append_generated_file_paths(&mut output, outcome.generated_file_paths());
            append_query_summaries(&mut output, outcome.query_summaries());
        }
    }

    output
}

const fn pluralize(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

fn append_generated_file_paths(output: &mut String, paths: &[std::path::PathBuf]) {
    if paths.is_empty() {
        output.push_str("Generated files: none.\n");
        return;
    }

    output.push_str("Generated files:\n");
    for path in paths {
        writeln!(output, "  - {}", path.display()).expect("writing to String cannot fail");
    }
}

fn append_query_summaries(output: &mut String, summaries: &[app::QuerySummary]) {
    if summaries.is_empty() {
        output.push_str("Queries: none.\n");
        return;
    }

    output.push_str("Queries:\n");
    for summary in summaries {
        writeln!(
            output,
            "  - {} ({}): {} {}",
            summary.id(),
            display_source_path(summary.source_path()),
            summary.param_count(),
            pluralize(summary.param_count(), "param", "params")
        )
        .expect("writing to String cannot fail");
    }
}

fn display_source_path(path: Option<&Path>) -> String {
    path.map_or_else(
        || "unknown source".to_owned(),
        |path| path.display().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn formats_check_success_summary_with_dry_run_details() {
        let outcome = ConfiguredCommandOutcome::Check(app::CheckOutcome::new(
            core::DiagnosticReport::default(),
            2,
            PathBuf::from("/tmp/project/src/generated/sqlcomp"),
            vec![
                app::QuerySummary::new(
                    "listUsers".to_owned(),
                    Some(PathBuf::from("sql/users.sql")),
                    0,
                ),
                app::QuerySummary::new(
                    "findUser".to_owned(),
                    Some(PathBuf::from("sql/users.sql")),
                    1,
                ),
            ],
        ));

        let summary = format_success_summary(&outcome);

        assert!(summary.contains("Check passed."));
        assert!(summary.contains("Matched 2 SQL files."));
        assert!(summary.contains("Compiled 2 queries."));
        assert!(summary.contains("Output dir: /tmp/project/src/generated/sqlcomp"));
        assert!(summary.contains("No files written."));
        assert!(summary.contains("- listUsers (sql/users.sql): 0 params"));
        assert!(summary.contains("- findUser (sql/users.sql): 1 param"));
    }

    #[test]
    fn formats_compile_success_summary_with_generated_paths() {
        let outcome = ConfiguredCommandOutcome::Compile(app::CompileOutcome::new(
            core::DiagnosticReport::default(),
            1,
            PathBuf::from("/tmp/project/src/generated/sqlcomp"),
            vec![app::QuerySummary::new(
                "listUsers".to_owned(),
                Some(PathBuf::from("sql/users.sql")),
                0,
            )],
            vec![PathBuf::from(
                "/tmp/project/src/generated/sqlcomp/sql/users.ts",
            )],
            Some(1),
        ));

        let summary = format_success_summary(&outcome);

        assert!(summary.contains("Compile succeeded."));
        assert!(summary.contains("Matched 1 SQL file."));
        assert!(summary.contains("Compiled 1 query."));
        assert!(summary.contains("Generated or updated 1 file."));
        assert!(summary.contains("Removed 1 stale generated file."));
        assert!(summary.contains("Generated files:"));
        assert!(summary.contains("- /tmp/project/src/generated/sqlcomp/sql/users.ts"));
        assert!(summary.contains("- listUsers (sql/users.sql): 0 params"));
    }
}
