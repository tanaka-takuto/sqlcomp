# Rust Coverage Gate

`script/check-coverage.sh` runs the external-service-free Rust coverage check:

```sh
script/check-coverage.sh
```

The script requires `cargo-llvm-cov` and gates on total line coverage percentage
using
`cargo llvm-cov --workspace --all-targets --all-features --fail-under-lines`.
CI installs `cargo-llvm-cov` 0.8.7.

The current minimum line coverage threshold is 85%. New Rust code should normally
add tests so total line coverage does not fall below that threshold.

The script writes an LCOV report to `coverage/lcov.info`. CI feeds that file to
octocov so pull requests receive a coverage report comment. octocov uses the same
85% acceptable threshold in [`.octocov.yml`](../.octocov.yml).
