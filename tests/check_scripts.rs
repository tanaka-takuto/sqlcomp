use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const DATABASE_URL: &str = "mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp";

#[test]
fn example_check_typechecks_temporary_generated_project() {
    let fixture = ScriptFixture::new("sqlcomp-check-examples");

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
    let fixture = ScriptFixture::new("sqlcomp-check-mysql-fixtures");

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
    let fixture = ScriptFixture::new("sqlcomp-check-coverage");

    let output = fixture.run_script("script/check-coverage.sh");

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
            .env("SQLCOMP_REPO_ROOT", &repo_root)
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
      copy_generated "$SQLCOMP_REPO_ROOT/examples/bookstore/generated" "$project_dir"
      exit 0
    fi

    project_dir=$(CDPATH= cd "../.." && pwd)
    copy_generated "$SQLCOMP_REPO_ROOT/fixtures/sql/generated" "$project_dir"
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
  "$TMPDIR"/sqlcomp-examples.*/bookstore/tsconfig.json) ;;
  "$TMPDIR"/sqlcomp-mysql-fixtures.*/sql/tsconfig.json) ;;
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
