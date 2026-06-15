use std::fs;
use std::path::Path;

#[test]
fn workspace_crate_roots_are_focused_facades() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crate_roots = [
        "crates/adapters/src/lib.rs",
        "crates/app/src/lib.rs",
        "crates/cli/src/lib.rs",
        "crates/core/src/lib.rs",
    ];
    let max_facade_lines = 120;
    let mut oversized_roots = Vec::new();

    for crate_root in crate_roots {
        let contents = fs::read_to_string(repo_root.join(crate_root))
            .unwrap_or_else(|error| panic!("failed to read {crate_root}: {error}"));
        let line_count = contents.lines().count();

        if line_count > max_facade_lines {
            oversized_roots.push(format!("{crate_root} has {line_count} lines"));
        }
    }

    assert!(
        oversized_roots.is_empty(),
        "crate roots should stay focused facades:\n{}",
        oversized_roots.join("\n")
    );
}
