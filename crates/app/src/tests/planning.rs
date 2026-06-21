use super::support::*;
use super::*;

#[test]
fn planner_resolves_config_paths_from_config_directory() {
    let config_dir = PathBuf::from("/tmp/sqlay-project/packages/api");
    let config = project_config(config_dir.clone());

    let plan = DefaultCompilationPlanner
        .plan(&config)
        .expect("valid config should produce a plan");

    assert_eq!(plan.config_dir(), config_dir);
    assert_eq!(plan.source_include(), [config_dir.join("sql/**/*.sql")]);
    assert_eq!(
        plan.source_exclude(),
        [config_dir.join("sql/private/**/*.sql")]
    );
    assert_eq!(plan.output_dir(), config_dir.join("src/generated/sqlay"));
    assert_eq!(plan.database(), config.database());
    assert_eq!(plan.target(), config.target());
}

#[test]
fn source_relative_path_uses_config_directory() {
    let config_dir = PathBuf::from("/tmp/sqlay-project");
    let config = project_config(config_dir.clone());
    let plan = DefaultCompilationPlanner
        .plan(&config)
        .expect("valid config should produce a plan");

    let relative_path = plan
        .source_relative_path(config_dir.join("packages/api/sql/users/list.sql"))
        .expect("source path should be inside config dir");

    assert_eq!(relative_path, Path::new("packages/api/sql/users/list.sql"));
}

#[test]
fn source_relative_path_rejects_paths_outside_config_directory() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let plan = DefaultCompilationPlanner
        .plan(&config)
        .expect("valid config should produce a plan");

    assert_eq!(
        plan.source_relative_path("/tmp/other-project/sql/users.sql"),
        None
    );
}

#[test]
fn source_read_carries_fragment_source_units() {
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1\n".to_owned(),
    )
    .with_source_path("sql/fragments.sql");

    let source_read = SourceRead::from_queries(Vec::new()).with_fragments(vec![fragment.clone()]);

    assert!(source_read.queries().is_empty());
    assert_eq!(source_read.fragments(), [fragment]);
}
