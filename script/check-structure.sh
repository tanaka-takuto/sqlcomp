#!/bin/sh

set -eu

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=${SQLCOMP_REPO_ROOT:-$(CDPATH= cd "$script_dir/.." && pwd)}
baseline_path=$repo_root/docs/structure-baseline.json

require_command() {
  command_name=$1
  install_hint=$2

  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
check-structure: $command_name is required.

Install:
  $install_hint
EOF
  exit 1
}

require_command "python3" "install Python 3 from https://www.python.org/"

python3 - "$repo_root" "$baseline_path" <<'PY'
from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any


DEFAULT_THRESHOLDS: dict[str, dict[str, int]] = {
    "production": {"soft": 600, "hard": 900},
    "test": {"soft": 900, "hard": 1300},
    "mod": {"soft": 250, "hard": 500},
    "directory": {"softFileCount": 8},
}
GENERIC_RUST_MODULE_NAMES = {"common.rs", "helpers.rs", "utils.rs"}
SOURCE_ROOTS = ("crates", "src", "tests")


def main() -> int:
    repo_root = Path(sys.argv[1]).resolve()
    baseline_path = Path(sys.argv[2]).resolve()
    failures: list[str] = []

    if not baseline_path.is_file():
        print(
            f"check-structure: missing baseline file: {baseline_path}",
            file=sys.stderr,
        )
        return 1

    baseline = load_baseline(baseline_path, failures)
    thresholds = load_thresholds(baseline, failures)
    file_baselines = load_file_baselines(baseline, failures)
    directory_baselines = load_directory_baselines(baseline, failures)

    rust_files = find_rust_files(repo_root)
    rust_file_paths = {relative_path(repo_root, path) for path in rust_files}

    for baseline_file in file_baselines:
        if baseline_file not in rust_file_paths:
            failures.append(
                f"{baseline_file} is listed in docs/structure-baseline.json but does not exist"
            )

    for path in rust_files:
        rel = relative_path(repo_root, path)
        kind = classify_file(rel)
        line_count = count_lines(path)
        threshold = thresholds[kind]
        baseline_entry = file_baselines.get(rel)

        if path.name in GENERIC_RUST_MODULE_NAMES:
            failures.append(
                f"{rel} uses generic module name {path.name}; use a responsibility-specific name"
            )

        if baseline_entry is not None:
            baseline_line_count = baseline_entry["lineCount"]
            if line_count > baseline_line_count:
                failures.append(
                    f"{rel} grew beyond baseline: {line_count} lines > {baseline_line_count}"
                )
            if line_count > threshold["hard"] and not baseline_entry.get(
                "allowAboveHardLimit", False
            ):
                failures.append(
                    f"{rel} exceeds {kind} hard limit {threshold['hard']} and needs "
                    "allowAboveHardLimit with a splitPlan in docs/structure-baseline.json"
                )
            continue

        if line_count > threshold["hard"]:
            failures.append(
                f"{rel} has {line_count} lines and exceeds {kind} hard limit "
                f"{threshold['hard']}; split it before merging"
            )
        elif line_count > threshold["soft"]:
            failures.append(
                f"{rel} has {line_count} lines and exceeds {kind} soft limit "
                f"{threshold['soft']}; split it or add a docs/structure-baseline.json "
                "entry with a splitPlan"
            )

    directory_counts = count_rust_files_by_directory(repo_root, rust_files)
    for baseline_directory in directory_baselines:
        if baseline_directory not in directory_counts:
            failures.append(
                f"{baseline_directory} is listed in docs/structure-baseline.json but has no Rust files"
            )

    directory_soft_limit = thresholds["directory"]["softFileCount"]
    for rel, file_count in sorted(directory_counts.items()):
        baseline_entry = directory_baselines.get(rel)
        if baseline_entry is not None:
            baseline_file_count = baseline_entry["fileCount"]
            if file_count > baseline_file_count:
                failures.append(
                    f"{rel} directory grew beyond baseline: "
                    f"{file_count} Rust files > {baseline_file_count}"
                )
            continue

        if file_count > directory_soft_limit:
            failures.append(
                f"{rel} contains {file_count} Rust files and exceeds directory soft limit "
                f"{directory_soft_limit}; group related modules into a private subdirectory "
                "or add a baseline splitPlan"
            )

    if failures:
        print("check-structure: failed", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(file=sys.stderr)
        print(
            "check-structure: prefer private submodules with responsibility-specific names. "
            "Keep crate boundaries unchanged unless an ADR changes them.",
            file=sys.stderr,
        )
        return 1

    print(
        "check-structure: ok "
        f"({len(rust_files)} Rust files, {len(directory_counts)} Rust directories)"
    )
    return 0


def load_baseline(path: Path, failures: list[str]) -> dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
        data = json.loads(raw)
    except OSError as error:
        print(f"check-structure: failed to read {path}: {error}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as error:
        print(f"check-structure: invalid JSON in {path}: {error}", file=sys.stderr)
        sys.exit(1)

    if not isinstance(data, dict):
        failures.append("docs/structure-baseline.json must contain a JSON object")
        return {}

    if data.get("version") != 1:
        failures.append("docs/structure-baseline.json must set version to 1")

    return data


def load_thresholds(
    baseline: dict[str, Any],
    failures: list[str],
) -> dict[str, dict[str, int]]:
    thresholds = {
        category: values.copy() for category, values in DEFAULT_THRESHOLDS.items()
    }
    configured = baseline.get("thresholds", {})
    if not isinstance(configured, dict):
        failures.append("thresholds must be a JSON object")
        return thresholds

    for category, values in configured.items():
        if category not in thresholds:
            failures.append(f"unknown threshold category: {category}")
            continue
        if not isinstance(values, dict):
            failures.append(f"thresholds.{category} must be a JSON object")
            continue
        for key, value in values.items():
            if key not in thresholds[category]:
                failures.append(f"unknown threshold key: thresholds.{category}.{key}")
                continue
            if not is_positive_int(value):
                failures.append(
                    f"thresholds.{category}.{key} must be a positive integer"
                )
                continue
            thresholds[category][key] = value

    return thresholds


def load_file_baselines(
    baseline: dict[str, Any],
    failures: list[str],
) -> dict[str, dict[str, Any]]:
    entries = baseline.get("files", [])
    if not isinstance(entries, list):
        failures.append("files must be a JSON array")
        return {}

    file_baselines: dict[str, dict[str, Any]] = {}
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            failures.append(f"files[{index}] must be a JSON object")
            continue
        path = entry.get("path")
        line_count = entry.get("lineCount")
        split_plan = entry.get("splitPlan")
        if not is_relative_repository_path(path):
            failures.append(f"files[{index}].path must be a relative repository path")
            continue
        if path in file_baselines:
            failures.append(f"duplicate file baseline path: {path}")
            continue
        if not is_positive_int(line_count):
            failures.append(f"files[{index}].lineCount must be a positive integer")
            continue
        if not isinstance(split_plan, str) or not split_plan.strip():
            failures.append(f"files[{index}].splitPlan must be a non-empty string")
            continue
        if "kind" in entry and entry["kind"] not in {"production", "test", "mod"}:
            failures.append(
                f"files[{index}].kind must be production, test, or mod when provided"
            )
            continue
        if "allowAboveHardLimit" in entry and not isinstance(
            entry["allowAboveHardLimit"], bool
        ):
            failures.append(f"files[{index}].allowAboveHardLimit must be boolean")
            continue
        file_baselines[path] = entry

    return file_baselines


def load_directory_baselines(
    baseline: dict[str, Any],
    failures: list[str],
) -> dict[str, dict[str, Any]]:
    entries = baseline.get("directories", [])
    if not isinstance(entries, list):
        failures.append("directories must be a JSON array")
        return {}

    directory_baselines: dict[str, dict[str, Any]] = {}
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            failures.append(f"directories[{index}] must be a JSON object")
            continue
        path = entry.get("path")
        file_count = entry.get("fileCount")
        split_plan = entry.get("splitPlan")
        if not is_relative_repository_path(path):
            failures.append(
                f"directories[{index}].path must be a relative repository path"
            )
            continue
        if path in directory_baselines:
            failures.append(f"duplicate directory baseline path: {path}")
            continue
        if not is_positive_int(file_count):
            failures.append(f"directories[{index}].fileCount must be a positive integer")
            continue
        if not isinstance(split_plan, str) or not split_plan.strip():
            failures.append(f"directories[{index}].splitPlan must be a non-empty string")
            continue
        directory_baselines[path] = entry

    return directory_baselines


def find_rust_files(repo_root: Path) -> list[Path]:
    rust_files: list[Path] = []
    for source_root in SOURCE_ROOTS:
        root = repo_root / source_root
        if not root.exists():
            continue
        rust_files.extend(path for path in root.rglob("*.rs") if path.is_file())
    return sorted(rust_files, key=lambda path: relative_path(repo_root, path))


def count_rust_files_by_directory(
    repo_root: Path,
    rust_files: list[Path],
) -> dict[str, int]:
    counts: Counter[str] = Counter()
    for path in rust_files:
        counts[relative_path(repo_root, path.parent)] += 1
    return dict(counts)


def classify_file(relative_file_path: str) -> str:
    if relative_file_path.endswith("/mod.rs"):
        return "mod"
    if (
        relative_file_path.startswith("tests/")
        or "/tests/" in relative_file_path
        or relative_file_path.endswith("/tests.rs")
        or relative_file_path.endswith("_test.rs")
    ):
        return "test"
    return "production"


def count_lines(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def relative_path(repo_root: Path, path: Path) -> str:
    return path.resolve().relative_to(repo_root).as_posix()


def is_relative_repository_path(value: Any) -> bool:
    if not isinstance(value, str) or value == "":
        return False
    path = Path(value)
    return not path.is_absolute() and ".." not in path.parts


def is_positive_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


if __name__ == "__main__":
    sys.exit(main())
PY
