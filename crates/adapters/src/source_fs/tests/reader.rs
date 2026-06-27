use std::fs;
use std::path::Path;

use sqlay_app::{DialectAnalyzer, SourceReader};
use sqlay_core as core;

use crate::dialect_mysql::MysqlDialectAnalyzer;

use super::super::FileSystemSourceReader;
use super::{
    assert_duplicate_query_report, assert_duplicate_source_unit_report, compilation_plan,
    diagnostic_messages, test_project_dir, write_sql,
};

#[test]
fn filesystem_source_reader_reads_included_files_as_query_blocks() {
    let project_dir = test_project_dir("reads-included-files");
    let sql_dir = project_dir.join("sql");
    fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
    fs::write(
        sql_dir.join("users.sql"),
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
/* @sqlay
{
  type: query
  id: findUser
  cardinality: one
}
*/
SELECT id FROM users WHERE id = 1;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline"),
    )
    .expect("test SQL file should be written");

    let source_read = FileSystemSourceReader
        .read(&compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        ))
        .expect("included SQL file should be read");
    let queries = source_read.queries();

    assert_eq!(source_read.source_file_count(), 1);
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].metadata().id(), "listUsers");
    assert_eq!(queries[0].metadata().cardinality(), None);
    assert_eq!(queries[0].sql(), "\nSELECT id FROM users;\n");
    assert_eq!(queries[1].metadata().id(), "findUser");
    assert_eq!(
        queries[1].metadata().cardinality(),
        Some(core::Cardinality::One)
    );
    assert_eq!(queries[1].sql(), "\nSELECT id FROM users WHERE id = 1;\n");

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_reads_fragment_only_files_without_unannotated_sql_warning() {
    let project_dir = test_project_dir("reads-fragment-only-files");
    let source_path = project_dir.join("sql").join("fragments.sql");
    write_sql(
        &source_path,
        r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1
",
    );
    let plan = compilation_plan(&project_dir, vec![source_path], Vec::new());

    let source_read = FileSystemSourceReader
        .read(&plan)
        .expect("fragment-only SQL file should be read");

    assert_eq!(source_read.source_file_count(), 1);
    assert!(source_read.queries().is_empty());
    assert_eq!(source_read.fragments().len(), 1);
    assert_eq!(source_read.fragments()[0].metadata().id(), "activeOnly");
    assert_eq!(
        source_read.fragments()[0].source_path(),
        Some(Path::new("sql/fragments.sql"))
    );
    assert!(source_read.diagnostics().is_empty());

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_reads_mutation_source_units_with_source_order() {
    let project_dir = test_project_dir("reads-mutation-source-units");
    let source_path = project_dir.join("sql").join("users.sql");
    write_sql(
        &source_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
/* @sqlay
{
  type: mutation
  id: createUser
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
    );
    let plan = compilation_plan(&project_dir, vec![source_path.clone()], Vec::new());

    let source_read = FileSystemSourceReader
        .read(&plan)
        .expect("mutation SQL file should be read");

    assert_eq!(source_read.source_file_count(), 1);
    assert_eq!(source_read.queries().len(), 1);
    assert_eq!(source_read.mutations().len(), 1);
    assert_eq!(source_read.source_units().len(), 2);
    assert!(matches!(
        source_read.source_units()[0],
        core::RawSourceUnit::Query(_)
    ));
    assert!(matches!(
        source_read.source_units()[1],
        core::RawSourceUnit::Mutation(_)
    ));
    assert_eq!(source_read.source_units()[0].id(), "listUsers");
    assert_eq!(source_read.source_units()[1].id(), "createUser");
    assert_eq!(
        source_read.mutations()[0].source_path(),
        Some(Path::new("sql/users.sql"))
    );
    assert_eq!(
        source_read.mutations()[0]
            .source_location()
            .and_then(core::SourceLocation::path),
        Some(source_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_does_not_collect_fragments_from_excluded_files() {
    let project_dir = test_project_dir("excludes-fragment-files");
    let query_path = project_dir.join("sql").join("users.sql");
    let fragment_path = project_dir
        .join("sql")
        .join("private")
        .join("fragments.sql");
    write_sql(
        &query_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT u.id
FROM users AS u
WHERE 1 = 1
/* @sqlay { type: slot id: filter targets: [privateFilter] } */;
",
    );
    write_sql(
        &fragment_path,
        r"
/* @sqlay
{
  type: fragment
  id: privateFilter
}
*/
AND u.private = 0
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        vec![project_dir.join("sql/private/**/*.sql")],
    );

    let source_read = FileSystemSourceReader
        .read(&plan)
        .expect("excluded fragment files should not be read");

    assert_eq!(source_read.source_file_count(), 1);
    assert_eq!(source_read.queries().len(), 1);
    assert!(source_read.fragments().is_empty());
    assert_eq!(
        source_read.queries()[0].slot_usages()[0].targets(),
        ["privateFilter"]
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_matches_question_mark_globs_and_deduplicates_sources() {
    let project_dir = test_project_dir("question-mark-glob-dedupes");
    let matched_path = project_dir.join("sql").join("user1.sql");
    let unmatched_path = project_dir.join("sql").join("user10.sql");
    let ignored_path = project_dir.join("sql").join("notes.txt");
    write_sql(
        &matched_path,
        r"
/* @sqlay
{
  type: query
  id: findUser1
}
*/
SELECT id FROM users WHERE id = 1;
",
    );
    write_sql(
        &unmatched_path,
        r"
/* @sqlay
{
  type: query
  id: findUser10
}
*/
SELECT id FROM users WHERE id = 10;
",
    );
    fs::write(&ignored_path, "not sql").expect("test text file should be written");
    let plan = compilation_plan(
        &project_dir,
        vec![
            project_dir.join("sql/./user?.sql"),
            project_dir.join("sql/user?.sql"),
            project_dir.join("sql/*.txt"),
        ],
        Vec::new(),
    );

    let source_read = FileSystemSourceReader
        .read(&plan)
        .expect("question-mark glob should discover SQL files");

    assert_eq!(source_read.source_file_count(), 1);
    assert_eq!(source_read.queries().len(), 1);
    assert_eq!(source_read.queries()[0].metadata().id(), "findUser1");
    assert_eq!(
        source_read.queries()[0].source_path(),
        Some(Path::new("sql/user1.sql"))
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_rejects_source_files_outside_config_dir() {
    let project_dir = test_project_dir("outside-config-root");
    let outside_dir = test_project_dir("outside-config-source");
    let outside_path = outside_dir.join("users.sql");
    write_sql(
        &outside_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
    );
    let plan = compilation_plan(&project_dir, vec![outside_path.clone()], Vec::new());

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("outside source files should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert!(diagnostic.message().starts_with(&format!(
        "source file `{}` is outside the configuration directory `{}`",
        outside_path.display(),
        project_dir.display()
    )));
    assert_eq!(
        diagnostic.location().and_then(core::SourceLocation::path),
        Some(outside_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
    fs::remove_dir_all(outside_dir).expect("outside test directory should be removed");
}

#[test]
fn filesystem_source_reader_attaches_file_path_to_scan_diagnostics() {
    let project_dir = test_project_dir("scan-diagnostic-path");
    let source_path = project_dir.join("sql").join("broken.sql");
    let parent = source_path
        .parent()
        .expect("test path should include a parent directory");
    fs::create_dir_all(parent).expect("test SQL directory should be created");
    fs::write(
        &source_path,
        r"/* @sqlay
{
  type: query
  id: brokenQuery
}
SELECT id FROM users;
",
    )
    .expect("broken SQL file should be written");
    let plan = compilation_plan(&project_dir, vec![source_path.clone()], Vec::new());

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("unterminated sqlay block should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert_eq!(diagnostic.message(), "unterminated SQL block comment");
    assert_eq!(
        diagnostic.location().and_then(core::SourceLocation::path),
        Some(source_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_attaches_file_path_to_query_locations() {
    let project_dir = test_project_dir("attaches-query-locations");
    let sql_dir = project_dir.join("sql");
    let sql_path = sql_dir.join("users.sql");
    fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
    fs::write(
        &sql_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline"),
    )
    .expect("test SQL file should be written");

    let source_read = FileSystemSourceReader
        .read(&compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        ))
        .expect("included SQL file should be read");
    let queries = source_read.queries();
    let location = queries[0]
        .source_location()
        .expect("query should include source location");
    let range = location
        .range()
        .expect("query should include SQL body range");

    assert_eq!(location.path(), Some(sql_path.as_path()));
    assert_eq!(range.start().line(), 7);
    assert_eq!(range.start().column(), 1);
    assert_eq!(queries[0].source_path(), Some(Path::new("sql/users.sql")));

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn filesystem_source_reader_attaches_file_path_to_repeat_locations() {
    let project_dir = test_project_dir("attaches-repeat-locations");
    let sql_path = project_dir.join("sql").join("users.sql");
    write_sql(
        &sql_path,
        r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
    );

    let source_read = FileSystemSourceReader
        .read(&compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        ))
        .expect("included SQL file should be read");
    let repeat = &source_read.queries()[0].repeat_usages()[0];
    let item_param = &repeat.item_param_usages()[0];

    assert_eq!(repeat.source_location().path(), Some(sql_path.as_path()));
    assert_eq!(
        item_param.source_location().path(),
        Some(sql_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_locations_feed_mysql_parser_diagnostics() {
    let project_dir = test_project_dir("feeds-parser-diagnostics");
    let sql_dir = project_dir.join("sql");
    let sql_path = sql_dir.join("users.sql");
    fs::create_dir_all(&sql_dir).expect("test SQL directory should be created");
    fs::write(
        &sql_path,
        r"
/* @sqlay
{
  type: query
  id: brokenQuery
}
*/
SELECT FROM;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline"),
    )
    .expect("test SQL file should be written");

    let source_read = FileSystemSourceReader
        .read(&compilation_plan(
            &project_dir,
            vec![project_dir.join("sql/**/*.sql")],
            Vec::new(),
        ))
        .expect("included SQL file should be read");
    let queries = source_read.queries();
    let report = MysqlDialectAnalyzer
        .analyze(&queries[0])
        .expect_err("invalid SQL should produce a parser diagnostic");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("parser diagnostic should be returned");
    let location = diagnostic
        .location()
        .expect("parser diagnostic should include source location");
    let range = location
        .range()
        .expect("parser diagnostic should include source range");

    assert!(
        diagnostic
            .message()
            .starts_with("failed to parse MySQL SQL:"),
        "message: {}",
        diagnostic.message()
    );
    assert_eq!(location.path(), Some(sql_path.as_path()));
    assert_eq!(range.start().line(), 7);
    assert_eq!(range.start().column(), 1);

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_rejects_duplicate_query_ids_in_the_same_file() {
    let project_dir = test_project_dir("duplicate-same-file");
    let source_path = project_dir.join("sql").join("users.sql");
    write_sql(
        &source_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;

/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
    );
    let plan = compilation_plan(&project_dir, vec![source_path.clone()], Vec::new());

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("duplicate query ids should be rejected");

    assert_duplicate_query_report(&report, &source_path);
    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_rejects_duplicate_query_ids_across_files() {
    let project_dir = test_project_dir("duplicate-across-files");
    let first_path = project_dir.join("sql").join("first.sql");
    let second_path = project_dir.join("sql").join("second.sql");
    write_sql(
        &first_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
    );
    write_sql(
        &second_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        Vec::new(),
    );

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("duplicate query ids should be rejected");

    assert_duplicate_query_report(&report, &second_path);
    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_rejects_duplicate_fragment_ids_across_files() {
    let project_dir = test_project_dir("duplicate-fragments-across-files");
    let first_path = project_dir.join("sql").join("01_first.sql");
    let second_path = project_dir.join("sql").join("02_second.sql");
    write_sql(
        &first_path,
        r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1
",
    );
    write_sql(
        &second_path,
        r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.deleted_at IS NULL
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        Vec::new(),
    );

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("duplicate fragment ids should be rejected");

    assert_duplicate_source_unit_report(
        &report,
        &second_path,
        "duplicate fragment id `activeOnly`; query, mutation, and fragment IDs must be unique across the full compile run",
    );
    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_rejects_query_and_fragment_id_collisions_across_files() {
    let project_dir = test_project_dir("duplicate-query-fragment-across-files");
    let query_path = project_dir.join("sql").join("01_query.sql");
    let fragment_path = project_dir.join("sql").join("02_fragment.sql");
    write_sql(
        &query_path,
        r"
/* @sqlay
{
  type: query
  id: activeOnly
}
*/
SELECT id FROM users;
",
    );
    write_sql(
        &fragment_path,
        r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        Vec::new(),
    );

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("query and fragment id collisions should be rejected");

    assert_duplicate_source_unit_report(
        &report,
        &fragment_path,
        "duplicate source unit id `activeOnly`; query, mutation, and fragment IDs must be unique across the full compile run",
    );
    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_rejects_query_mutation_and_fragment_id_collisions_across_files() {
    let project_dir = test_project_dir("duplicate-query-mutation-fragment-across-files");
    let query_path = project_dir.join("sql").join("01_query.sql");
    let mutation_path = project_dir.join("sql").join("02_mutation.sql");
    let fragment_path = project_dir.join("sql").join("03_fragment.sql");
    write_sql(
        &query_path,
        r"
/* @sqlay
{
  type: query
  id: sharedUnit
}
*/
SELECT id FROM users;
",
    );
    write_sql(
        &mutation_path,
        r"
/* @sqlay
{
  type: mutation
  id: sharedUnit
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
    );
    write_sql(
        &fragment_path,
        r"
/* @sqlay
{
  type: fragment
  id: sharedUnit
}
*/
AND u.active = 1
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        Vec::new(),
    );

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("query, mutation, and fragment ID collisions should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        [
            "duplicate source unit id `sharedUnit`; query, mutation, and fragment IDs must be unique across the full compile run",
            "first declared here",
            "duplicate source unit id `sharedUnit`; query, mutation, and fragment IDs must be unique across the full compile run",
            "first declared here",
        ]
    );
    assert_eq!(
        report.diagnostics()[0]
            .location()
            .and_then(core::SourceLocation::path),
        Some(mutation_path.as_path())
    );
    assert_eq!(
        report.diagnostics()[1]
            .location()
            .and_then(core::SourceLocation::path),
        Some(query_path.as_path())
    );
    assert_eq!(
        report.diagnostics()[2]
            .location()
            .and_then(core::SourceLocation::path),
        Some(fragment_path.as_path())
    );
    assert_eq!(
        report.diagnostics()[3]
            .location()
            .and_then(core::SourceLocation::path),
        Some(query_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_reports_same_file_source_unit_collisions_in_source_order() {
    let project_dir = test_project_dir("duplicate-source-unit-same-file-source-order");
    let source_path = project_dir.join("sql").join("users.sql");
    write_sql(
        &source_path,
        r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1

/* @sqlay
{
  type: query
  id: activeOnly
}
*/
SELECT id FROM users;
",
    );
    let plan = compilation_plan(&project_dir, vec![source_path.clone()], Vec::new());

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("same-file query and fragment ID collision should be rejected");

    assert_duplicate_source_unit_report(
        &report,
        &source_path,
        "duplicate source unit id `activeOnly`; query, mutation, and fragment IDs must be unique across the full compile run",
    );
    assert_eq!(
        report.diagnostics()[0]
            .location()
            .and_then(core::SourceLocation::range)
            .map(|range| range.start().line()),
        Some(15)
    );
    assert_eq!(
        report.diagnostics()[1]
            .location()
            .and_then(core::SourceLocation::range)
            .map(|range| range.start().line()),
        Some(7)
    );
    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}

#[test]
fn source_reader_collects_independent_source_intake_diagnostics_across_files() {
    let project_dir = test_project_dir("aggregates-source-intake-diagnostics");
    let exec_path = project_dir.join("sql").join("01_exec_cardinality.sql");
    let first_duplicate_path = project_dir.join("sql").join("02_duplicate_first.sql");
    let second_duplicate_path = project_dir.join("sql").join("03_duplicate_second.sql");
    write_sql(
        &exec_path,
        r"
/* @sqlay
{
  type: query
  id: execQuery
  cardinality: exec
}
*/
SELECT id FROM users;
",
    );
    write_sql(
        &first_duplicate_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
    );
    write_sql(
        &second_duplicate_path,
        r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM archived_users;
",
    );
    let plan = compilation_plan(
        &project_dir,
        vec![project_dir.join("sql/**/*.sql")],
        Vec::new(),
    );

    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("source intake diagnostics should be aggregated");

    assert_eq!(
        diagnostic_messages(&report),
        [
            "`cardinality: exec` is reserved for future non-SELECT support and is not currently supported",
            "duplicate query id `listUsers`; query, mutation, and fragment IDs must be unique across the full compile run",
            "first declared here",
        ]
    );
    assert_eq!(
        report.diagnostics()[0]
            .location()
            .and_then(core::SourceLocation::path),
        Some(exec_path.as_path())
    );
    assert_eq!(
        report.diagnostics()[1]
            .location()
            .and_then(core::SourceLocation::path),
        Some(second_duplicate_path.as_path())
    );
    assert_eq!(
        report.diagnostics()[2]
            .location()
            .and_then(core::SourceLocation::path),
        Some(first_duplicate_path.as_path())
    );

    fs::remove_dir_all(project_dir).expect("test project directory should be removed");
}
