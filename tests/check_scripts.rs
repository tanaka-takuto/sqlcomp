use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

const DATABASE_URL: &str = "mysql://sqlay:sqlay@127.0.0.1:3306/sqlay";

#[test]
fn example_check_typechecks_temporary_generated_project() {
    let fixture = ScriptFixture::new("sqlay-check-examples");

    let output = fixture.run_script("script/check-examples.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mysql_fixture_check_typechecks_temporary_generated_project() {
    let fixture = ScriptFixture::new("sqlay-check-mysql-fixtures");

    let output = fixture.run_script("script/check-mysql-fixtures.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn coverage_check_uses_line_percent_threshold_and_writes_lcov() {
    let fixture = ScriptFixture::new("sqlay-check-coverage");

    let output = fixture.run_script("script/check-coverage.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn structure_check_accepts_committed_baseline() {
    let fixture = ScriptFixture::new("sqlay-check-structure");

    let output = fixture.run_script("script/check-structure.sh");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn structure_check_rejects_unbaselined_large_source_file() {
    let fixture = ScriptFixture::new("sqlay-check-structure-large-file");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(
        &repo.join("crates/app/src/new_large.rs"),
        &rust_comment_lines("// production line ", 601),
    );

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail for an unbaselined large file"
    );
    assert!(
        stderr.contains("crates/app/src/new_large.rs"),
        "stderr should identify the large file: {stderr}"
    );
    assert!(
        stderr.contains("exceeds production soft limit"),
        "stderr should explain the threshold failure: {stderr}"
    );
}

#[test]
fn structure_check_rejects_baseline_growth() {
    let fixture = ScriptFixture::new("sqlay-check-structure-ratchet");
    let repo = fixture.root.join("repo");
    write_structure_baseline(
        &repo,
        r#"{
  "version": 1,
  "files": [
    {
      "path": "crates/app/src/lib.rs",
      "lineCount": 2,
      "kind": "production",
      "splitPlan": "Keep this test fixture small."
    }
  ],
  "directories": []
}"#,
    );
    write_file(
        &repo.join("crates/app/src/lib.rs"),
        "// one\n// two\n// three\n",
    );

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail when a baselined file grows"
    );
    assert!(
        stderr.contains("grew beyond baseline"),
        "stderr should describe the ratchet failure: {stderr}"
    );
}

#[test]
fn structure_check_rejects_unbaselined_large_module_directory() {
    let fixture = ScriptFixture::new("sqlay-check-structure-large-directory");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    for index in 0..9 {
        write_file(
            &repo.join(format!("crates/app/src/module_{index}.rs")),
            "// small module\n",
        );
    }

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail when a module directory grows too wide"
    );
    assert!(
        stderr.contains("crates/app/src"),
        "stderr should identify the wide module directory: {stderr}"
    );
    assert!(
        stderr.contains("private subdirectory"),
        "stderr should suggest directory splitting: {stderr}"
    );
}

#[test]
fn structure_check_rejects_generic_module_names() {
    let fixture = ScriptFixture::new("sqlay-check-structure-generic-name");
    let repo = fixture.root.join("repo");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(&repo.join("crates/app/src/utils.rs"), "// escape hatch\n");

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "structure check should fail for generic module names"
    );
    assert!(
        stderr.contains("uses generic module name utils.rs"),
        "stderr should identify the forbidden filename: {stderr}"
    );
}

#[test]
fn structure_check_ignores_symlinked_rust_files_outside_repo() {
    let fixture = ScriptFixture::new("sqlay-check-structure-external-symlink");
    let repo = fixture.root.join("repo");
    let outside = fixture.root.join("outside.rs");
    write_structure_baseline(&repo, r#"{"version":1,"files":[],"directories":[]}"#);
    write_file(&outside, "// outside repo\n");
    std::fs::create_dir_all(repo.join("crates/app/src"))
        .expect("fixture module directory should be created");
    symlink(&outside, repo.join("crates/app/src/outside.rs"))
        .expect("fixture symlink should be created");

    let output = fixture.run_script_with_repo_root("script/check-structure.sh", &repo);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct ScriptFixture {
    root: PathBuf,
    fake_bin: PathBuf,
    home: PathBuf,
}

impl ScriptFixture {
    fn new(prefix: &str) -> Self {
        let root = unique_temp_dir(prefix);
        let fake_bin = root.join("bin");
        let home = root.join("home");
        std::fs::create_dir_all(&fake_bin).expect("fake bin directory should be created");
        std::fs::create_dir_all(&home).expect("fake home directory should be created");

        let fixture = Self {
            root,
            fake_bin,
            home,
        };
        fixture.write_fake_cargo();
        fixture.write_fake_mysql();
        fixture.write_fake_npm();
        fixture
    }

    fn run_script(&self, script_path: &str) -> std::process::Output {
        self.run_script_with_repo_root(script_path, &repo_root())
    }

    fn run_script_with_repo_root(
        &self,
        script_path: &str,
        target_repo_root: &Path,
    ) -> std::process::Output {
        let repo_root = repo_root();
        let path = format!(
            "{}:{}",
            self.fake_bin.display(),
            std::env::var("PATH").expect("PATH should be set")
        );

        Command::new(repo_root.join(script_path))
            .env("DATABASE_URL", DATABASE_URL)
            .env("HOME", &self.home)
            .env("PATH", path)
            .env("SQLAY_REPO_ROOT", target_repo_root)
            .env("TMPDIR", &self.root)
            .output()
            .expect("check script should run")
    }

    fn write_fake_cargo(&self) {
        write_executable(
            &self.fake_bin.join("cargo"),
            r#"#!/bin/sh
set -eu

copy_generated() {
  expected_dir=$1
  project_dir=$2

  mkdir -p "$project_dir/generated"
  cp -R "$expected_dir/." "$project_dir/generated/"
}

case "$1" in
  run)
    config_path=
    while [ "$#" -gt 0 ]; do
      if [ "$1" = "--config" ]; then
        config_path=$2
        break
      fi
      shift
    done

    if [ -n "$config_path" ]; then
      project_dir=$(CDPATH= cd "$(dirname "$config_path")" && pwd)
      copy_generated "$SQLAY_REPO_ROOT/examples/bookstore/generated" "$project_dir"
      exit 0
    fi

    project_dir=$(CDPATH= cd "../.." && pwd)
    copy_generated "$SQLAY_REPO_ROOT/fixtures/sql/generated" "$project_dir"
    ;;
  test)
    ;;
  llvm-cov)
    if [ "$#" -eq 2 ] && [ "$2" = "--version" ]; then
      exit 0
    fi

    expected_args="llvm-cov --workspace --all-targets --all-features --fail-under-lines 85 --lcov --output-path coverage/lcov.info"
    if [ "$*" != "$expected_args" ]; then
      echo "expected cargo coverage args: $expected_args, got: $*" >&2
      exit 64
    fi
    ;;
  *)
    echo "unexpected cargo args: $*" >&2
    exit 64
    ;;
esac
"#,
        );
    }

    fn write_fake_mysql(&self) {
        write_executable(
            &self.fake_bin.join("mysql"),
            r"#!/bin/sh
cat >/dev/null
",
        );
    }

    fn write_fake_npm(&self) {
        write_executable(
            &self.fake_bin.join("npm"),
            r#"#!/bin/sh
set -eu

if [ "$#" -ne 6 ] \
  || [ "$1" != "exec" ] \
  || [ "$2" != "--" ] \
  || [ "$3" != "tsc" ] \
  || [ "$4" != "--noEmit" ] \
  || [ "$5" != "--project" ]; then
  echo "expected npm to typecheck a temporary generated project, got: $*" >&2
  exit 64
fi

case "$6" in
  "$TMPDIR"/sqlay-examples.*/bookstore/tsconfig.json) ;;
  "$TMPDIR"/sqlay-mysql-fixtures.*/sql/tsconfig.json) ;;
  *)
    echo "expected npm to typecheck a temporary generated project, got: $*" >&2
    exit 64
    ;;
esac
"#,
        );
    }
}

impl Drop for ScriptFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn write_executable(path: &Path, content: &str) {
    let mut file = std::fs::File::create(path).expect("fake command should be created");
    file.write_all(content.as_bytes())
        .expect("fake command should be written");
    let mut permissions = file
        .metadata()
        .expect("fake command metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fake command should be executable");
}

fn write_file(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture path should have a parent"))
        .expect("fixture parent directory should be created");
    std::fs::write(path, content).expect("fixture file should be written");
}

fn write_structure_baseline(repo_root: &Path, content: &str) {
    write_file(&repo_root.join("docs/structure-baseline.json"), content);
}

fn rust_comment_lines(prefix: &str, line_count: usize) -> String {
    let mut content = String::new();
    for line in 0..line_count {
        content.push_str(prefix);
        content.push_str(&line.to_string());
        content.push('\n');
    }
    content
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}
