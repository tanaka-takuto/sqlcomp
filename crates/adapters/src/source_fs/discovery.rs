use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use sqlcomp_core as core;

use crate::source_fs::diagnostics::file_error;

pub(super) fn discover_source_files(
    plan: &core::CompilationPlan,
) -> core::DiagnosticResult<Vec<PathBuf>> {
    let mut files = BTreeSet::new();

    for include in plan.source_include() {
        for path in files_matching_pattern(include)? {
            if is_sql_file(&path) && !is_excluded(&path, plan.source_exclude()) {
                files.insert(path);
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn files_matching_pattern(pattern: &Path) -> core::DiagnosticResult<Vec<PathBuf>> {
    if !path_has_glob(pattern) {
        return Ok(pattern
            .is_file()
            .then(|| pattern.to_path_buf())
            .into_iter()
            .collect());
    }

    let root = static_glob_prefix(pattern);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_matching_files(&root, pattern, &mut files)?;
    Ok(files)
}

fn collect_matching_files(
    directory: &Path,
    pattern: &Path,
    files: &mut Vec<PathBuf>,
) -> core::DiagnosticResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| {
            file_error(
                format!(
                    "failed to read source directory `{}`: {error}",
                    directory.display()
                ),
                directory,
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            file_error(
                format!(
                    "failed to read an entry in source directory `{}`: {error}",
                    directory.display()
                ),
                directory,
            )
        })?;

    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            file_error(
                format!(
                    "failed to inspect source path `{}`: {error}",
                    path.display()
                ),
                &path,
            )
        })?;

        if file_type.is_dir() {
            collect_matching_files(&path, pattern, files)?;
        } else if file_type.is_file() && path_matches_pattern(&path, pattern) {
            files.push(path);
        }
    }

    Ok(())
}

fn static_glob_prefix(pattern: &Path) -> PathBuf {
    let mut prefix = PathBuf::new();

    for component in pattern.components() {
        if component_has_glob(component) {
            break;
        }
        prefix.push(component.as_os_str());
    }

    if prefix.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        prefix
    }
}

fn path_matches_pattern(path: &Path, pattern: &Path) -> bool {
    let path_components = normalized_path_components(path);
    let pattern_components = normalized_path_components(pattern);

    path_components_match(&pattern_components, &path_components)
}

fn path_components_match(pattern: &[String], path: &[String]) -> bool {
    match (pattern.split_first(), path.split_first()) {
        (None, None) => true,
        (Some((component, remaining_pattern)), _) if component == "**" => {
            path_components_match(remaining_pattern, path)
                || path.split_first().is_some_and(|(_, remaining_path)| {
                    path_components_match(pattern, remaining_path)
                })
        }
        (Some((component, remaining_pattern)), Some((path_component, remaining_path))) => {
            component_matches_pattern(component, path_component)
                && path_components_match(remaining_pattern, remaining_path)
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn component_matches_pattern(pattern: &str, value: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let value = value.chars().collect::<Vec<_>>();

    component_chars_match(&pattern, &value)
}

fn component_chars_match(pattern: &[char], value: &[char]) -> bool {
    match (pattern.split_first(), value.split_first()) {
        (None, None) => true,
        (Some(('*', remaining_pattern)), _) => {
            component_chars_match(remaining_pattern, value)
                || value.split_first().is_some_and(|(_, remaining_value)| {
                    component_chars_match(pattern, remaining_value)
                })
        }
        (Some(('?', remaining_pattern)), Some((_, remaining_value))) => {
            component_chars_match(remaining_pattern, remaining_value)
        }
        (Some((pattern_char, remaining_pattern)), Some((value_char, remaining_value))) => {
            pattern_char == value_char && component_chars_match(remaining_pattern, remaining_value)
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn normalized_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir => Some(String::new()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_owned()),
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
        })
        .collect()
}

fn is_excluded(path: &Path, exclude_patterns: &[PathBuf]) -> bool {
    exclude_patterns
        .iter()
        .any(|pattern| path_matches_pattern(path, pattern))
}

fn is_sql_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "sql")
}

fn path_has_glob(path: &Path) -> bool {
    path.components().any(component_has_glob)
}

fn component_has_glob(component: Component<'_>) -> bool {
    component
        .as_os_str()
        .to_string_lossy()
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?'))
}
