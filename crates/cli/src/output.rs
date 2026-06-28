use sqlay_app as app;
use sqlay_core as core;
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
                "Check passed. Matched {} SQL {}. Compiled {} {}. Resolved {} {}. Resolved {} unique {}. Resolved {} unique {}. Validated {} {}. Output dir: {}. No files written.",
                outcome.source_file_count(),
                pluralize(outcome.source_file_count(), "file", "files"),
                outcome.builder_count(),
                format_builder_breakdown(outcome.query_count(), outcome.mutation_count()),
                outcome.fragment_count(),
                pluralize(outcome.fragment_count(), "fragment", "fragments"),
                outcome.unique_slot_count(),
                pluralize(outcome.unique_slot_count(), "slot", "slots"),
                outcome.unique_repeat_count(),
                pluralize(outcome.unique_repeat_count(), "repeat", "repeats"),
                outcome.validation_case_count(),
                pluralize(
                    outcome.validation_case_count(),
                    "validation case",
                    "validation cases"
                ),
                outcome.output_dir().display()
            )
            .expect("writing to String cannot fail");
            append_query_summaries(&mut output, outcome.query_summaries());
            append_mutation_summaries(&mut output, outcome.mutation_summaries());
        }
        ConfiguredCommandOutcome::Compile(outcome) => {
            write!(
                &mut output,
                "Compile succeeded. Matched {} SQL {}. Compiled {} {}. Resolved {} {}. Resolved {} unique {}. Resolved {} unique {}. Validated {} {}. Generated or updated {} {}.",
                outcome.source_file_count(),
                pluralize(outcome.source_file_count(), "file", "files"),
                outcome.builder_count(),
                format_builder_breakdown(outcome.query_count(), outcome.mutation_count()),
                outcome.fragment_count(),
                pluralize(outcome.fragment_count(), "fragment", "fragments"),
                outcome.unique_slot_count(),
                pluralize(outcome.unique_slot_count(), "slot", "slots"),
                outcome.unique_repeat_count(),
                pluralize(outcome.unique_repeat_count(), "repeat", "repeats"),
                outcome.validation_case_count(),
                pluralize(
                    outcome.validation_case_count(),
                    "validation case",
                    "validation cases"
                ),
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
            append_mutation_summaries(&mut output, outcome.mutation_summaries());
        }
    }

    output
}

fn format_builder_breakdown(query_count: usize, mutation_count: usize) -> String {
    format!(
        "{}: {} {}, {} {}",
        pluralize(query_count + mutation_count, "builder", "builders"),
        query_count,
        pluralize(query_count, "query", "queries"),
        mutation_count,
        pluralize(mutation_count, "mutation", "mutations")
    )
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
            "  - {} ({}): {}",
            summary.id(),
            display_source_path(summary.source_path()),
            format_query_detail_summary(summary)
        )
        .expect("writing to String cannot fail");
    }
}

fn append_mutation_summaries(output: &mut String, summaries: &[app::MutationSummary]) {
    if summaries.is_empty() {
        output.push_str("Mutations: none.\n");
        return;
    }

    output.push_str("Mutations:\n");
    for summary in summaries {
        writeln!(
            output,
            "  - {} ({}): {}",
            summary.id(),
            display_source_path(summary.source_path()),
            format_mutation_detail_summary(summary)
        )
        .expect("writing to String cannot fail");
    }
}

fn format_query_detail_summary(summary: &app::QuerySummary) -> String {
    let parameter_summary = if summary.param_count() == 0 {
        "no parameters".to_owned()
    } else {
        format!(
            "{} {}, {} {}",
            summary.param_count(),
            pluralize(
                summary.param_count(),
                "parameter placeholder",
                "parameter placeholders"
            ),
            summary.input_field_count(),
            pluralize(summary.input_field_count(), "input field", "input fields")
        )
    };

    format!(
        "{parameter_summary}, {} {}, {} {}, {} {}",
        summary.slot_count(),
        pluralize(summary.slot_count(), "slot", "slots"),
        summary.repeat_count(),
        pluralize(summary.repeat_count(), "repeat", "repeats"),
        summary.validation_case_count(),
        pluralize(
            summary.validation_case_count(),
            "validation case",
            "validation cases"
        )
    )
}

fn format_mutation_detail_summary(summary: &app::MutationSummary) -> String {
    let parameter_summary = if summary.param_count() == 0 {
        "no parameters".to_owned()
    } else {
        format!(
            "{} {}, {} {}",
            summary.param_count(),
            pluralize(
                summary.param_count(),
                "parameter placeholder",
                "parameter placeholders"
            ),
            summary.input_field_count(),
            pluralize(summary.input_field_count(), "input field", "input fields")
        )
    };

    format!(
        "{}, {parameter_summary}, {} {}, {} {}, {} {}",
        mutation_kind_name(summary.kind()),
        summary.slot_count(),
        pluralize(summary.slot_count(), "slot", "slots"),
        summary.repeat_count(),
        pluralize(summary.repeat_count(), "repeat", "repeats"),
        summary.validation_case_count(),
        pluralize(
            summary.validation_case_count(),
            "validation case",
            "validation cases"
        )
    )
}

const fn mutation_kind_name(kind: core::MutationKind) -> &'static str {
    match kind {
        core::MutationKind::Insert => "insert",
        core::MutationKind::Update => "update",
        core::MutationKind::Delete => "delete",
        core::MutationKind::Replace => "replace",
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
            PathBuf::from("/tmp/project/src/generated/sqlay"),
            vec![
                app::QuerySummary::new(
                    "listUsers".to_owned(),
                    Some(PathBuf::from("sql/users.sql")),
                    app::BuilderSummaryCounts::new(0, 0, 0, 0, 1),
                ),
                app::QuerySummary::new(
                    "filterUsers".to_owned(),
                    Some(PathBuf::from("sql/users.sql")),
                    app::BuilderSummaryCounts::new(3, 2, 0, 1, 1),
                ),
            ],
            vec![app::MutationSummary::new(
                "bulkCreateUsers".to_owned(),
                Some(PathBuf::from("sql/users.sql")),
                core::MutationKind::Insert,
                app::BuilderSummaryCounts::new(2, 1, 0, 1, 1),
            )],
            0,
        ));

        let summary = format_success_summary(&outcome);

        assert!(summary.contains("Check passed."));
        assert!(summary.contains("Matched 2 SQL files."));
        assert!(summary.contains("Compiled 3 builders: 2 queries, 1 mutation."));
        assert!(summary.contains("Resolved 0 fragments."));
        assert!(summary.contains("Resolved 0 unique slots."));
        assert!(summary.contains("Resolved 2 unique repeats."));
        assert!(summary.contains("Validated 3 validation cases."));
        assert!(summary.contains("Output dir: /tmp/project/src/generated/sqlay"));
        assert!(summary.contains("No files written."));
        assert!(summary.contains(
            "- listUsers (sql/users.sql): no parameters, 0 slots, 0 repeats, 1 validation case"
        ));
        assert!(
            summary.contains(
                "- filterUsers (sql/users.sql): 3 parameter placeholders, 2 input fields, 0 slots, 1 repeat, 1 validation case"
            )
        );
        assert!(
            summary.contains(
                "- bulkCreateUsers (sql/users.sql): insert, 2 parameter placeholders, 1 input field, 0 slots, 1 repeat, 1 validation case"
            )
        );
    }

    #[test]
    fn formats_compile_success_summary_with_generated_paths() {
        let outcome = ConfiguredCommandOutcome::Compile(
            app::CompileOutcome::new(
                core::DiagnosticReport::default(),
                1,
                PathBuf::from("/tmp/project/src/generated/sqlay"),
                vec![app::QuerySummary::new(
                    "listUsers".to_owned(),
                    Some(PathBuf::from("sql/users.sql")),
                    app::BuilderSummaryCounts::new(0, 0, 0, 0, 1),
                )],
                Vec::new(),
                vec![PathBuf::from(
                    "/tmp/project/src/generated/sqlay/sql/users.ts",
                )],
                0,
            )
            .with_stale_file_removal_count(Some(1)),
        );

        let summary = format_success_summary(&outcome);

        assert!(summary.contains("Compile succeeded."));
        assert!(summary.contains("Matched 1 SQL file."));
        assert!(summary.contains("Compiled 1 builder: 1 query, 0 mutations."));
        assert!(summary.contains("Resolved 0 fragments."));
        assert!(summary.contains("Resolved 0 unique slots."));
        assert!(summary.contains("Resolved 0 unique repeats."));
        assert!(summary.contains("Validated 1 validation case."));
        assert!(summary.contains("Generated or updated 1 file."));
        assert!(summary.contains("Removed 1 stale generated file."));
        assert!(summary.contains("Generated files:"));
        assert!(summary.contains("- /tmp/project/src/generated/sqlay/sql/users.ts"));
        assert!(summary.contains(
            "- listUsers (sql/users.sql): no parameters, 0 slots, 0 repeats, 1 validation case"
        ));
        assert!(summary.contains("Mutations: none."));
    }
}
