use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use sqlay_app::TargetGenerator;
use sqlay_core as core;

use super::builders::render_generated_file_contents_from_iter;

/// TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(
        &self,
        plan: &core::CompilationPlan,
        builders: &[core::CompiledBuilder],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        let mut builders_by_source_path: BTreeMap<PathBuf, Vec<&core::CompiledBuilder>> =
            BTreeMap::new();

        for builder in builders {
            let source_path = builder_source_path(builder)?;
            builders_by_source_path
                .entry(source_path.to_path_buf())
                .or_default()
                .push(builder);
        }

        let mut files = Vec::with_capacity(builders_by_source_path.len());
        for (source_path, source_builders) in builders_by_source_path {
            let output_path = generated_typescript_path(plan.output_dir(), &source_path);
            let source_queries = collect_supported_queries(&source_builders)?;
            let contents = render_generated_file_contents_from_iter(source_queries);
            files.push(core::GeneratedFile::new(output_path, contents));
        }

        Ok(core::GeneratedFiles::new(files))
    }
}

fn builder_source_path(builder: &core::CompiledBuilder) -> core::DiagnosticResult<&Path> {
    let Some(source_path) = builder.source_path() else {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled builder `{}` does not include a source file path for output mapping",
                builder.id()
            ),
        )));
    };

    if !is_safe_relative_path(source_path) {
        return Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "compiled builder `{}` has invalid source file path `{}`; expected a config-relative SQL path",
                builder.id(),
                source_path.display()
            ),
        )));
    }

    Ok(source_path)
}

fn collect_supported_queries<'a>(
    builders: &[&'a core::CompiledBuilder],
) -> core::DiagnosticResult<Vec<&'a core::CompiledQuery>> {
    let mut queries = Vec::with_capacity(builders.len());

    for builder in builders {
        match *builder {
            core::CompiledBuilder::Query(query) => queries.push(query),
            core::CompiledBuilder::Mutation(mutation) => {
                return Err(core::DiagnosticReport::new(core::Diagnostic::error(
                    format!(
                        "compiled mutation `{}` reached TypeScript generation, but TypeScript mutation builder generation is not implemented yet",
                        mutation.id().as_str()
                    ),
                )));
            }
        }
    }

    Ok(queries)
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
