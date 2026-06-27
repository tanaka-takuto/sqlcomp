mod inline_markers;
mod metadata;
mod reader;
mod repeat_markers;
mod scanner;
mod source_units;

use std::fs;
use std::path::{Path, PathBuf};

use sqlay_core as core;

fn assert_duplicate_query_report(report: &core::DiagnosticReport, duplicate_path: &Path) {
    assert_duplicate_source_unit_report(
        report,
        duplicate_path,
        "duplicate query id `listUsers`; query, mutation, and fragment IDs must be unique across the full compile run",
    );
}

fn assert_duplicate_source_unit_report(
    report: &core::DiagnosticReport,
    duplicate_path: &Path,
    expected_message: &str,
) {
    assert_eq!(report.diagnostics().len(), 2);
    assert_eq!(report.diagnostics()[0].message(), expected_message);
    assert_eq!(
        report.diagnostics()[0]
            .location()
            .and_then(core::SourceLocation::path),
        Some(duplicate_path)
    );
    assert_eq!(report.diagnostics()[1].message(), "first declared here");
}

fn diagnostic_messages(report: &core::DiagnosticReport) -> Vec<&str> {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect()
}

fn compilation_plan(
    config_dir: &Path,
    source_include: Vec<PathBuf>,
    source_exclude: Vec<PathBuf>,
) -> core::CompilationPlan {
    core::CompilationPlan::new(
        config_dir.to_path_buf(),
        source_include,
        source_exclude,
        config_dir.join("generated"),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    )
}

fn write_sql(path: &Path, contents: &str) {
    let contents = contents
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
    let parent = path.parent().expect("test path should include a parent");
    fs::create_dir_all(parent).expect("temp source dir should be created");
    fs::write(path, contents).expect("temp SQL file should be written");
}

fn test_project_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlay-source-fs-{name}-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("stale test project directory should be removed");
    }
    fs::create_dir_all(&dir).expect("test project directory should be created");
    dir
}
