use std::path::PathBuf;

const OCTOCOV_ACTION_SHA: &str = "b3b6ee60482a667950f87553abf1df63217235d9";

#[test]
fn coverage_workflow_uses_pinned_octocov_action() {
    let workflow =
        std::fs::read_to_string(repo_root().join(".github/workflows/_coverage-check.yml"))
            .expect("coverage workflow should be readable");

    assert!(
        workflow.contains(&format!("k1LoW/octocov-action@{OCTOCOV_ACTION_SHA}")),
        "octocov action should be pinned to a full commit SHA"
    );
    assert!(
        !workflow.contains("k1LoW/octocov-action@v1"),
        "octocov action must not be pinned to a moving tag"
    );
}

#[test]
fn octocov_config_comments_line_coverage_report_with_threshold() {
    let config = std::fs::read_to_string(repo_root().join(".octocov.yml"))
        .expect("octocov config should be readable");

    assert!(config.contains("coverage/lcov.info"));
    assert!(config.contains("acceptable: 85%"));
    assert!(config.contains("if: is_pull_request"));
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
