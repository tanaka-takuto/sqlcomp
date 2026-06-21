use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sqlay_app::{SourceRead, SourceReader};
use sqlay_core as core;

use crate::source_fs::diagnostics::{
    attach_path, contains_non_comment_sql, extend_diagnostics, file_error, unannotated_sql_warning,
};
use crate::source_fs::discovery::discover_source_files;
use crate::source_fs::scanner::scan_sqlay_blocks;
use crate::source_fs::source_units::split_sqlay_source_units_from_scan;

/// Filesystem-backed source reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemSourceReader;

impl SourceReader for FileSystemSourceReader {
    fn read(&self, plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        let mut seen_ids = HashMap::new();
        let mut queries = Vec::new();
        let mut fragments = Vec::new();
        let mut diagnostics = core::DiagnosticReport::default();
        let mut fatal_diagnostics = core::DiagnosticReport::default();
        let source_files = discover_source_files(plan)?;
        let source_file_count = source_files.len();

        for path in source_files {
            let Some(source_path) = plan.source_relative_path(&path) else {
                extend_diagnostics(
                    &mut fatal_diagnostics,
                    file_error(
                        format!(
                            "source file `{}` is outside the configuration directory `{}`; source.include paths are resolved from the config file directory and must stay inside it so generated paths can be preserved relative to that directory under output.dir. Move sqlay.config.json to a common project root when SQL lives in sibling directories.",
                            path.display(),
                            plan.config_dir().display()
                        ),
                        &path,
                    ),
                );
                continue;
            };
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    extend_diagnostics(
                        &mut fatal_diagnostics,
                        file_error(
                            format!(
                                "failed to read SQL source file `{}`: {error}",
                                path.display()
                            ),
                            &path,
                        ),
                    );
                    continue;
                }
            };
            let scan = match scan_sqlay_blocks(&source) {
                Ok(scan) => scan,
                Err(report) => {
                    extend_diagnostics(&mut fatal_diagnostics, attach_path(report, &path));
                    continue;
                }
            };
            if scan.blocks().is_empty() && contains_non_comment_sql(scan.sql_without_sqlay_blocks())
            {
                diagnostics.push(unannotated_sql_warning(&path));
            }

            let source_units = match split_sqlay_source_units_from_scan(&source, &scan) {
                Ok(source_units) => source_units,
                Err(report) => {
                    extend_diagnostics(&mut fatal_diagnostics, attach_path(report, &path));
                    continue;
                }
            };
            let (file_queries, file_fragments) = source_units.into_parts();
            let file_queries = file_queries
                .into_iter()
                .map(|query| attach_query_path(query, &path).with_source_path(source_path.clone()))
                .collect::<Vec<_>>();
            let file_fragments = file_fragments
                .into_iter()
                .map(|fragment| {
                    attach_fragment_path(fragment, &path).with_source_path(source_path.clone())
                })
                .collect::<Vec<_>>();
            collect_duplicate_source_unit_ids(
                &file_queries,
                &file_fragments,
                &mut seen_ids,
                &mut fatal_diagnostics,
            );
            queries.extend(file_queries);
            fragments.extend(file_fragments);
        }

        if !fatal_diagnostics.is_empty() {
            return Err(fatal_diagnostics);
        }

        Ok(SourceRead::new(queries, diagnostics)
            .with_fragments(fragments)
            .with_source_file_count(source_file_count))
    }
}

fn attach_query_path(query: core::RawQuery, path: &Path) -> core::RawQuery {
    let range = query
        .source_location()
        .and_then(core::SourceLocation::range);
    let param_usages = query
        .param_usages()
        .iter()
        .cloned()
        .map(|usage| attach_param_usage_path(usage, path))
        .collect::<Vec<_>>();
    let slot_usages = query
        .slot_usages()
        .iter()
        .cloned()
        .map(|usage| attach_slot_usage_path(usage, path))
        .collect::<Vec<_>>();

    let query = if let Some(range) = range {
        query.with_source_location(core::SourceLocation::at_range(path, range))
    } else {
        query.with_source_location(core::SourceLocation::for_path(path))
    };

    query
        .with_param_usages(param_usages)
        .with_slot_usages(slot_usages)
}

fn attach_fragment_path(fragment: core::RawFragment, path: &Path) -> core::RawFragment {
    let range = fragment
        .source_location()
        .and_then(core::SourceLocation::range);
    let param_usages = fragment
        .param_usages()
        .iter()
        .cloned()
        .map(|usage| attach_param_usage_path(usage, path))
        .collect::<Vec<_>>();

    let fragment = if let Some(range) = range {
        fragment.with_source_location(core::SourceLocation::at_range(path, range))
    } else {
        fragment.with_source_location(core::SourceLocation::for_path(path))
    };

    fragment.with_param_usages(param_usages)
}

fn attach_param_usage_path(usage: core::ParamUsage, path: &Path) -> core::ParamUsage {
    if let Some(range) = usage.source_location().range() {
        usage.with_source_location(core::SourceLocation::at_range(path, range))
    } else {
        usage.with_source_location(core::SourceLocation::for_path(path))
    }
}

fn attach_slot_usage_path(usage: core::SlotUsage, path: &Path) -> core::SlotUsage {
    if let Some(range) = usage.source_location().range() {
        usage.with_source_location(core::SourceLocation::at_range(path, range))
    } else {
        usage.with_source_location(core::SourceLocation::for_path(path))
    }
}

struct SourceUnitDeclaration {
    kind: SourceUnitKind,
    location: Option<core::SourceLocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceUnitOccurrence<'a> {
    id: &'a str,
    kind: SourceUnitKind,
    location: Option<core::SourceLocation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceUnitKind {
    Query,
    Fragment,
}

type SeenSourceUnitIds = HashMap<String, SourceUnitDeclaration>;

fn collect_duplicate_source_unit_ids(
    queries: &[core::RawQuery],
    fragments: &[core::RawFragment],
    seen_ids: &mut SeenSourceUnitIds,
    diagnostics: &mut core::DiagnosticReport,
) {
    let mut source_units = Vec::with_capacity(queries.len() + fragments.len());

    for query in queries {
        source_units.push(SourceUnitOccurrence {
            id: query.metadata().id(),
            kind: SourceUnitKind::Query,
            location: query.source_location().cloned(),
        });
    }

    for fragment in fragments {
        source_units.push(SourceUnitOccurrence {
            id: fragment.metadata().id(),
            kind: SourceUnitKind::Fragment,
            location: fragment.source_location().cloned(),
        });
    }

    source_units.sort_by_key(|source_unit| source_unit_location_key(source_unit.location.as_ref()));

    for source_unit in source_units {
        collect_duplicate_source_unit_id(
            source_unit.id,
            source_unit.kind,
            source_unit.location,
            seen_ids,
            diagnostics,
        );
    }
}

fn source_unit_location_key(location: Option<&core::SourceLocation>) -> (usize, usize) {
    location
        .and_then(core::SourceLocation::range)
        .map_or((usize::MAX, usize::MAX), |range| {
            (range.start().line(), range.start().column())
        })
}

fn collect_duplicate_source_unit_id(
    id: &str,
    kind: SourceUnitKind,
    location: Option<core::SourceLocation>,
    seen_ids: &mut SeenSourceUnitIds,
    diagnostics: &mut core::DiagnosticReport,
) {
    let declaration = SourceUnitDeclaration {
        kind,
        location: location.clone(),
    };

    if let Some(first_declaration) = seen_ids.get(id) {
        diagnostics.push(
            core::Diagnostic::error(duplicate_source_unit_message(
                id,
                kind,
                first_declaration.kind,
            ))
            .with_location(location.unwrap_or_else(core::SourceLocation::unknown)),
        );
        diagnostics.push(
            core::Diagnostic::note("first declared here").with_location(
                first_declaration
                    .location
                    .clone()
                    .unwrap_or_else(core::SourceLocation::unknown),
            ),
        );
    } else {
        seen_ids.insert(id.to_owned(), declaration);
    }
}

fn duplicate_source_unit_message(
    id: &str,
    duplicate_kind: SourceUnitKind,
    first_kind: SourceUnitKind,
) -> String {
    match (first_kind, duplicate_kind) {
        (SourceUnitKind::Query, SourceUnitKind::Query) => {
            format!(
                "duplicate query id `{id}`; query IDs must be unique across the full compile run"
            )
        }
        (SourceUnitKind::Fragment, SourceUnitKind::Fragment) => {
            format!(
                "duplicate fragment id `{id}`; query and fragment IDs must be unique across the full compile run"
            )
        }
        _ => {
            format!(
                "duplicate source unit id `{id}`; query and fragment IDs must be unique across the full compile run"
            )
        }
    }
}
