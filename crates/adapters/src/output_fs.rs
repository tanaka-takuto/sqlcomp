//! Filesystem generated-output adapter.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sqlcomp_app::{GeneratedFileCleaner, GeneratedFileWriter};
use sqlcomp_core as core;

/// Filesystem-backed generated file writer and stale-file cleaner.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemGeneratedFileWriter;

impl GeneratedFileWriter for FileSystemGeneratedFileWriter {
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
        for file in files.files() {
            write_generated_file(file)?;
        }

        Ok(())
    }
}

fn write_generated_file(file: &core::GeneratedFile) -> core::DiagnosticResult<()> {
    if let Some(parent) = file
        .path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            file_error(
                format!(
                    "failed to create generated output directory `{}`: {error}",
                    parent.display()
                ),
                parent,
            )
        })?;
    }

    fs::write(file.path(), file.contents()).map_err(|error| {
        file_error(
            format!(
                "failed to write generated file `{}`: {error}",
                file.path().display()
            ),
            file.path(),
        )
    })
}

impl GeneratedFileCleaner for FileSystemGeneratedFileWriter {
    fn clean_stale(
        &self,
        output_dir: &Path,
        current_files: &core::GeneratedFiles,
    ) -> core::DiagnosticResult<()> {
        if !output_dir.try_exists().map_err(|error| {
            file_error(
                format!(
                    "failed to inspect generated output directory `{}`: {error}",
                    output_dir.display()
                ),
                output_dir,
            )
        })? {
            return Ok(());
        }

        let current_paths = CurrentGeneratedPaths::new(current_files)?;

        clean_stale_files_in_dir(output_dir, &current_paths)
    }
}

struct CurrentGeneratedPaths {
    literal: HashSet<PathBuf>,
    canonical: HashSet<PathBuf>,
}

impl CurrentGeneratedPaths {
    fn new(files: &core::GeneratedFiles) -> core::DiagnosticResult<Self> {
        let mut literal = HashSet::new();
        let mut canonical = HashSet::new();

        for file in files.files() {
            literal.insert(file.path().to_path_buf());
            canonical.insert(canonicalize_generated_path(file.path())?);
        }

        Ok(Self { literal, canonical })
    }

    fn contains(&self, path: &Path) -> core::DiagnosticResult<bool> {
        if self.literal.contains(path) {
            return Ok(true);
        }

        Ok(self.canonical.contains(&canonicalize_generated_path(path)?))
    }
}

fn clean_stale_files_in_dir(
    dir: &Path,
    current_paths: &CurrentGeneratedPaths,
) -> core::DiagnosticResult<()> {
    let entries = fs::read_dir(dir).map_err(|error| {
        file_error(
            format!(
                "failed to read generated output directory `{}`: {error}",
                dir.display()
            ),
            dir,
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            file_error(
                format!(
                    "failed to inspect generated output directory `{}`: {error}",
                    dir.display()
                ),
                dir,
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            file_error(
                format!(
                    "failed to inspect generated output path `{}`: {error}",
                    path.display()
                ),
                &path,
            )
        })?;

        if file_type.is_dir() {
            clean_stale_files_in_dir(&path, current_paths)?;
        } else if file_type.is_file()
            && !current_paths.contains(&path)?
            && is_managed_generated_file(&path)?
        {
            fs::remove_file(&path).map_err(|error| {
                file_error(
                    format!(
                        "failed to remove stale generated file `{}`: {error}",
                        path.display()
                    ),
                    &path,
                )
            })?;
        }
    }

    Ok(())
}

fn canonicalize_generated_path(path: &Path) -> core::DiagnosticResult<PathBuf> {
    fs::canonicalize(path).map_err(|error| {
        file_error(
            format!(
                "failed to inspect generated output path `{}`: {error}",
                path.display()
            ),
            path,
        )
    })
}

fn is_managed_generated_file(path: &Path) -> core::DiagnosticResult<bool> {
    let mut file = fs::File::open(path).map_err(|error| {
        file_error(
            format!(
                "failed to read generated file `{}` before cleanup: {error}",
                path.display()
            ),
            path,
        )
    })?;
    let mut header = vec![0; core::GENERATED_FILE_HEADER.len()];

    match file.read_exact(&mut header) {
        Ok(()) => Ok(header == core::GENERATED_FILE_HEADER.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(file_error(
            format!(
                "failed to read generated file `{}` before cleanup: {error}",
                path.display()
            ),
            path,
        )),
    }
}

fn file_error(message: impl Into<String>, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::for_path(path)),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::FileSystemGeneratedFileWriter;
    use sqlcomp_app::{GeneratedFileCleaner, GeneratedFileWriter};
    use sqlcomp_core as core;

    #[test]
    fn writes_generated_files_and_creates_parent_directories() {
        let output_dir = unique_temp_dir("write-parents");
        let file_path = output_dir.join("sql").join("admin").join("users.ts");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            file_path.clone(),
            "// @generated by sqlcomp. Do not edit.\n".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("writer should create parent directories and write file");

        assert_eq!(
            fs::read_to_string(&file_path).expect("generated file should be readable"),
            "// @generated by sqlcomp. Do not edit.\n"
        );

        fs::remove_dir_all(output_dir).expect("temp output dir should be removed");
    }

    #[test]
    fn overwrites_existing_same_path_files() {
        let output_dir = unique_temp_dir("overwrite");
        let file_path = output_dir.join("sql").join("users.ts");
        fs::create_dir_all(file_path.parent().expect("file path should have a parent"))
            .expect("temp parent directory should be created");
        fs::write(&file_path, "keep me").expect("existing file should be written");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            file_path.clone(),
            "// @generated by sqlcomp. Do not edit.\nexport {}\n".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("writer should overwrite generated output path");

        assert_eq!(
            fs::read_to_string(&file_path).expect("generated file should be readable"),
            "// @generated by sqlcomp. Do not edit.\nexport {}\n"
        );

        fs::remove_dir_all(output_dir).expect("temp output dir should be removed");
    }

    #[test]
    fn write_leaves_other_generated_files_untouched() {
        let output_dir = unique_temp_dir("write-keeps-stale");
        let current_path = output_dir.join("sql").join("users.ts");
        let stale_path = output_dir.join("sql").join("old_users.ts");
        fs::create_dir_all(stale_path.parent().expect("file path should have a parent"))
            .expect("temp parent directory should be created");
        fs::write(&stale_path, "// @generated by sqlcomp. Do not edit.\nold\n")
            .expect("stale file should be written");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            current_path,
            "// @generated by sqlcomp. Do not edit.\ncurrent\n".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("writer should write only current generated output");

        assert!(
            stale_path.exists(),
            "normal write should leave stale generated files untouched"
        );

        fs::remove_dir_all(output_dir).expect("temp output dir should be removed");
    }

    #[test]
    fn clean_stale_removes_only_managed_files_missing_from_current_outputs() {
        let output_dir = unique_temp_dir("clean-stale");
        let current_path = output_dir.join("sql").join("users.ts");
        let stale_managed_path = output_dir.join("sql").join("old_users.ts");
        let unmanaged_path = output_dir.join("sql").join("notes.ts");
        fs::create_dir_all(
            current_path
                .parent()
                .expect("file path should have a parent"),
        )
        .expect("temp parent directory should be created");
        fs::write(
            &current_path,
            "// @generated by sqlcomp. Do not edit.\ncurrent\n",
        )
        .expect("current managed file should be written");
        fs::write(
            &stale_managed_path,
            "// @generated by sqlcomp. Do not edit.\nstale\n",
        )
        .expect("stale managed file should be written");
        fs::write(&unmanaged_path, "export const keep = true;\n")
            .expect("unmanaged file should be written");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            current_path.clone(),
            "// @generated by sqlcomp. Do not edit.\ncurrent\n".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .clean_stale(&output_dir, &files)
            .expect("clean should remove stale managed generated files");

        assert!(current_path.exists(), "current managed file should remain");
        assert!(
            !stale_managed_path.exists(),
            "stale managed file should be removed"
        );
        assert!(unmanaged_path.exists(), "unmanaged file should remain");

        fs::remove_dir_all(output_dir).expect("temp output dir should be removed");
    }

    #[test]
    fn clean_stale_keeps_current_file_after_case_only_output_rename() {
        let output_dir = unique_temp_dir("clean-case-only-rename");
        let existing_path = output_dir.join("sql").join("users.ts");
        let current_path = output_dir.join("sql").join("Users.ts");
        fs::create_dir_all(
            existing_path
                .parent()
                .expect("file path should have a parent"),
        )
        .expect("temp parent directory should be created");
        fs::write(
            &existing_path,
            "// @generated by sqlcomp. Do not edit.\nstale\n",
        )
        .expect("existing generated file should be written");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            current_path.clone(),
            "// @generated by sqlcomp. Do not edit.\ncurrent\n".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("writer should write current generated file");
        FileSystemGeneratedFileWriter
            .clean_stale(&output_dir, &files)
            .expect("clean should keep the current generated file");

        assert!(
            current_path.exists(),
            "current generated file should remain after cleanup"
        );
        assert_eq!(
            fs::read_to_string(&current_path).expect("current file should be readable"),
            "// @generated by sqlcomp. Do not edit.\ncurrent\n"
        );

        fs::remove_dir_all(output_dir).expect("temp output dir should be removed");
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "sqlcomp-output-fs-{name}-{}-{unique}",
            std::process::id()
        ))
    }
}
