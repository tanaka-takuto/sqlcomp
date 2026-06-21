use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use sqlcomp_app::TargetGenerator;
use sqlcomp_core as core;

use super::builders::render_generated_file_contents_from_iter;

/// TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(
        &self,
        plan: &core::CompilationPlan,
        queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        let mut queries_by_source_path: BTreeMap<PathBuf, Vec<&core::CompiledQuery>> =
            BTreeMap::new();

        for query in queries {
            let source_path = query_source_path(query)?;
            queries_by_source_path
                .entry(source_path.to_path_buf())
                .or_default()
                .push(query);
        }

        let mut files = Vec::with_capacity(queries_by_source_path.len());
        for (source_path, source_queries) in queries_by_source_path {
            let output_path = generated_typescript_path(plan.output_dir(), &source_path);
            let contents = render_generated_file_contents_from_iter(source_queries);
            files.push(core::GeneratedFile::new(output_path, contents));
        }

        Ok(core::GeneratedFiles::new(files))
    }
}

fn query_source_path(query: &core::CompiledQuery) -> core::DiagnosticResult<&Path> {
    let Some(source_path) = query.source_path() else {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled query `{}` does not include a source file path for output mapping",
                query.id().as_str()
            ),
        )));
    };

    if !is_safe_relative_path(source_path) {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled query `{}` has invalid source file path `{}`; expected a config-relative SQL path",
                query.id().as_str(),
                source_path.display()
            ),
        )));
    }

    Ok(source_path)
}

fn generated_typescript_path(output_dir: &Path, source_relative_path: &Path) -> PathBuf {
    output_dir.join(source_relative_path).with_extension("ts")
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.file_name().is_some()
        && path
            .components()
            .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
}
