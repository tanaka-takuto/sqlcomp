//! User-facing diagnostic primitives shared across compilation components.

use std::error::Error;
use std::fmt;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

/// Convenient result alias for components that can emit user-facing diagnostics.
pub type DiagnosticResult<T> = std::result::Result<T, DiagnosticReport>;

/// Severity of a diagnostic message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    /// A failure that should stop the current command.
    Error,
    /// A non-fatal problem that should be shown to the user.
    Warning,
    /// Additional context for another diagnostic.
    Note,
}

impl DiagnosticSeverity {
    /// Return the stable CLI-facing severity label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One-based source position for user-facing diagnostics.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    line: NonZeroUsize,
    column: NonZeroUsize,
}

impl SourcePosition {
    /// Build a source position from non-zero one-based coordinates.
    #[must_use]
    pub const fn from_nonzero(line: NonZeroUsize, column: NonZeroUsize) -> Self {
        Self { line, column }
    }

    /// Build a source position from one-based coordinates.
    ///
    /// Returns `None` when either coordinate is zero.
    #[must_use]
    pub fn one_based(line: usize, column: usize) -> Option<Self> {
        Some(Self {
            line: NonZeroUsize::new(line)?,
            column: NonZeroUsize::new(column)?,
        })
    }

    /// One-based line number.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line.get()
    }

    /// One-based column number.
    #[must_use]
    pub const fn column(self) -> usize {
        self.column.get()
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line(), self.column())
    }
}

/// One-based source range for user-facing diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceRange {
    start: SourcePosition,
    end: Option<SourcePosition>,
}

impl SourceRange {
    /// Build a point source range.
    #[must_use]
    pub const fn point(position: SourcePosition) -> Self {
        Self {
            start: position,
            end: None,
        }
    }

    /// Build a source range with an optional end position.
    #[must_use]
    pub const fn new(start: SourcePosition, end: Option<SourcePosition>) -> Self {
        Self { start, end }
    }

    /// Starting source position.
    #[must_use]
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    /// Optional ending source position.
    #[must_use]
    pub const fn end(self) -> Option<SourcePosition> {
        self.end
    }
}

impl fmt::Display for SourceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(end) = self.end {
            write!(f, "{}-{end}", self.start)
        } else {
            write!(f, "{}", self.start)
        }
    }
}

/// Optional file path and source range attached to a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    path: Option<PathBuf>,
    range: Option<SourceRange>,
}

impl SourceLocation {
    /// Build a source location without file or range context.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            path: None,
            range: None,
        }
    }

    /// Build a source location for an entire path.
    #[must_use]
    pub fn for_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
            range: None,
        }
    }

    /// Build a source location at a single position within a path.
    #[must_use]
    pub fn at_position(path: impl Into<PathBuf>, position: SourcePosition) -> Self {
        Self::at_range(path, SourceRange::point(position))
    }

    /// Build a source location for a range within a path.
    #[must_use]
    pub fn at_range(path: impl Into<PathBuf>, range: SourceRange) -> Self {
        Self {
            path: Some(path.into()),
            range: Some(range),
        }
    }

    /// Build a source location for a range without path context.
    #[must_use]
    pub const fn from_range(range: SourceRange) -> Self {
        Self {
            path: None,
            range: Some(range),
        }
    }

    /// Return the diagnostic path when one is available.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Return the diagnostic source range when one is available.
    #[must_use]
    pub const fn range(&self) -> Option<SourceRange> {
        self.range
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.path.as_ref(), self.range) {
            (Some(path), Some(range)) => write!(f, "{}:{range}", path.display()),
            (Some(path), None) => write!(f, "{}", path.display()),
            (None, Some(range)) => write!(f, "{range}"),
            (None, None) => f.write_str("<unknown>"),
        }
    }
}

/// A structured user-facing diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    message: String,
    location: Option<SourceLocation>,
}

impl Diagnostic {
    /// Build a diagnostic with the provided severity and message.
    #[must_use]
    pub fn new(severity: DiagnosticSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            location: None,
        }
    }

    /// Build an error diagnostic.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, message)
    }

    /// Build a warning diagnostic.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Warning, message)
    }

    /// Build a note diagnostic.
    #[must_use]
    pub fn note(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Note, message)
    }

    /// Attach source location context.
    #[must_use]
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Diagnostic severity.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Human-readable diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Source location context when one is available.
    #[must_use]
    pub const fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(f, "{location}: {}: {}", self.severity, self.message)
        } else {
            write!(f, "{}: {}", self.severity, self.message)
        }
    }
}

/// One or more diagnostics that should be visible to command users.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticReport {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    /// Build a report from a single diagnostic.
    #[must_use]
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    /// Build a report from zero or more diagnostics.
    #[must_use]
    pub const fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    /// Append one diagnostic to the report.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Return all diagnostics in this report.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Consume the report and return its diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Return whether this report contains no diagnostics.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

impl From<Diagnostic> for DiagnosticReport {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::new(diagnostic)
    }
}

impl From<Vec<Diagnostic>> for DiagnosticReport {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self::from_diagnostics(diagnostics)
    }
}

impl fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.diagnostics.is_empty() {
            return f.write_str("error: no diagnostics");
        }

        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{diagnostic}")?;
        }

        Ok(())
    }
}

impl Error for DiagnosticReport {}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticReport, SourceLocation, SourcePosition, SourceRange};

    #[test]
    fn source_position_rejects_zero_coordinates() {
        assert_eq!(SourcePosition::one_based(0, 1), None);
        assert_eq!(SourcePosition::one_based(1, 0), None);
    }

    #[test]
    fn diagnostic_format_includes_location_severity_and_message() {
        let position = SourcePosition::one_based(12, 5).expect("test position should be valid");
        let diagnostic = Diagnostic::error("expected a SELECT statement")
            .with_location(SourceLocation::at_position("queries/users.sql", position));

        assert_eq!(
            diagnostic.to_string(),
            "queries/users.sql:12:5: error: expected a SELECT statement"
        );
    }

    #[test]
    fn source_location_can_format_ranges_without_paths() {
        let start = SourcePosition::one_based(3, 8).expect("test start position should be valid");
        let end = SourcePosition::one_based(3, 14).expect("test end position should be valid");
        let location = SourceLocation::from_range(SourceRange::new(start, Some(end)));

        assert_eq!(location.to_string(), "3:8-3:14");
    }

    #[test]
    fn diagnostic_converts_into_report() {
        let diagnostic = Diagnostic::error("missing database URL");
        let report = DiagnosticReport::from(diagnostic);

        assert_eq!(report.to_string(), "error: missing database URL");
    }

    #[test]
    fn report_formats_multiple_diagnostics_on_separate_lines() {
        let report = DiagnosticReport::from_diagnostics(vec![
            Diagnostic::error("invalid config"),
            Diagnostic::note("read sqlcomp.config.json"),
        ]);

        assert_eq!(
            report.to_string(),
            "error: invalid config\nnote: read sqlcomp.config.json"
        );
    }
}
