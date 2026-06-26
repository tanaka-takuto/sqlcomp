use std::path::Path;

use sqlay_core as core;

pub(super) fn extend_diagnostics(
    diagnostics: &mut core::DiagnosticReport,
    report: core::DiagnosticReport,
) {
    for diagnostic in report.into_diagnostics() {
        diagnostics.push(diagnostic);
    }
}

pub(super) fn file_error(message: impl Into<String>, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::for_path(path)),
    )
}

pub(super) fn attach_path(report: core::DiagnosticReport, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::from_diagnostics(
        report
            .into_diagnostics()
            .into_iter()
            .map(|diagnostic| {
                if diagnostic
                    .location()
                    .and_then(core::SourceLocation::path)
                    .is_some()
                {
                    return diagnostic;
                }

                let location = diagnostic
                    .location()
                    .and_then(core::SourceLocation::range)
                    .map_or_else(
                        || core::SourceLocation::for_path(path),
                        |range| core::SourceLocation::at_range(path, range),
                    );

                core::Diagnostic::new(diagnostic.severity(), diagnostic.message())
                    .with_location(location)
            })
            .collect(),
    )
}

pub(super) fn unannotated_sql_warning(path: &Path) -> core::Diagnostic {
    core::Diagnostic::warning(
        "included SQL file contains SQL but no `@sqlay` query or mutation annotation; add a `/* @sqlay { type: query, id: ... } */` block before a SELECT builder or `/* @sqlay { type: mutation, id: ... } */` before a mutation builder",
    )
    .with_location(core::SourceLocation::for_path(path))
}

pub(super) fn contains_non_comment_sql(source: &str) -> bool {
    NonCommentSqlScanner::new(source).contains_sql()
}

struct NonCommentSqlScanner<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> NonCommentSqlScanner<'a> {
    const fn new(source: &'a str) -> Self {
        Self { source, index: 0 }
    }

    fn contains_sql(mut self) -> bool {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(char::is_whitespace) {
                self.advance_current();
            } else {
                return true;
            }
        }

        false
    }

    fn skip_block_comment(&mut self) {
        self.advance_current();
        self.advance_current();

        while !self.is_at_end() {
            if self.starts_with("*/") {
                self.advance_current();
                self.advance_current();
                return;
            }

            self.advance_current();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(char) = self.advance_current() {
            if char == '\n' {
                return;
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.source.len()
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}

pub(super) fn metadata_error(
    message: impl Into<String>,
    range: core::SourceRange,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::from_range(range)),
    )
}
