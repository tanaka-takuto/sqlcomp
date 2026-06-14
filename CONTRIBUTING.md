# Contributing

Thanks for contributing to `sqlcomp`.

This repository uses GitHub issue templates, a pull request template, and local Git hooks to keep contributions consistent.

## Install local tools

Install dprint before setting up Git hooks:

```sh
brew install dprint
```

Install Rust with rustup:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup component add rustfmt clippy
```

Install Node.js and the pinned TypeScript dependency used by generated-code checks:

```sh
npm ci
```

Verify the local toolchain:

```sh
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
```

## Start local MySQL

The MVP uses MySQL 8.x for metadata checks. Start the development service with:

```sh
script/mysql-up.sh
```

The Compose service uses deterministic development-only credentials:

```sh
export DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp'
```

MySQL fixture DDL and seed data live in `fixtures/mysql/init/`. The official MySQL
image runs those files when a fresh database volume is created. To rebuild the
local fixture database from scratch:

```sh
script/mysql-reset.sh
```

Stop the service without removing the database volume:

```sh
script/mysql-down.sh
```

## Run local checks

Run the non-database baseline repository checks before opening a pull request:

```sh
script/check-all.sh
```

Run only the generated TypeScript type check with:

```sh
npm run typecheck:generated
```

Run the MySQL-backed integration checks against a running MySQL service:

```sh
DATABASE_URL='mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp' script/mysql-integration.sh
```

## Set up Git hooks

Run these commands once after cloning the repository:

```sh
git config core.hooksPath .githooks
git config commit.template .githooks/commit-message-template
chmod +x .githooks/commit-msg .githooks/pre-commit .githooks/pre-push
```

`core.hooksPath` makes Git use the hooks stored in this repository. `commit.template` pre-fills commit messages with the expected format and examples.

## Commit messages

Use Conventional Commits:

```text
type(scope): short summary
```

The scope is optional:

```text
feat: add query parser
fix(parser): handle empty input
docs: update setup steps
```

Allowed types:

- `build`: build system or dependency changes
- `chore`: maintenance tasks that do not change runtime behavior
- `ci`: CI configuration or scripts
- `docs`: documentation-only changes
- `feat`: new user-facing behavior
- `fix`: bug fixes
- `perf`: performance improvements
- `refactor`: code changes that preserve behavior
- `revert`: reverting a previous change
- `style`: formatting changes that do not affect behavior
- `test`: tests or test utilities

Use `!` before the colon for breaking changes:

```text
feat!: change public API shape
feat(parser)!: require explicit dialect
```

The `commit-msg` hook allows generated merge, revert, fixup, squash, and amend commits.

## Branch names

Branches used for issue-based pull requests must use this format:

```text
issue/#123
```

The `pre-push` hook checks branch names before pushing. It allows `main`, `master`, and `develop` for repository maintenance, and requires all other pushed local branches to match `issue/#<number>`.

## Issues

Use the GitHub issue form that best matches the request:

- Bug report
- Feature request
- Question

Blank issues are disabled so that reports include enough context to act on them.

## Pull requests

Before opening a pull request:

- Keep the change focused on one problem.
- Use a GitHub closing keyword such as `Closes #123` when the pull request should close an issue.
- Update documentation when behavior or setup changes.
- Run the relevant checks for the area you changed.
- Fill in the pull request template with the tests you ran and any reviewer notes.

## GitHub Actions layout

Formatting CI follows a three-layer GitHub Actions layout:

- Trigger layer: `.github/workflows/on_pull_request_format-check.yml` and `.github/workflows/on_push_format-check.yml`
- Reusable Workflow layer: `.github/workflows/_format-check.yml`
- Composite Action layer: `.github/actions/setup-dprint/action.yml`

Trigger workflows define when formatting CI runs. The reusable workflow owns the format-check job. The composite action installs the pinned dprint CLI version used by CI.

Rust CI follows the same layout:

- Trigger layer: `.github/workflows/on_pull_request_rust-check.yml` and `.github/workflows/on_push_rust-check.yml`
- Reusable Workflow layer: `.github/workflows/_rust-check.yml`
- Composite Action layer: `.github/actions/setup-rust/action.yml`

The Rust workflow runs:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features
cargo test --locked --workspace --all-targets --all-features
```

`script/check-all.sh` combines the format and Rust checks into one local baseline
command.

MySQL integration CI follows the same trigger and reusable workflow split:

- Trigger layer: `.github/workflows/on_pull_request_mysql-integration.yml` and `.github/workflows/on_push_mysql-integration.yml`
- Reusable Workflow layer: `.github/workflows/_mysql-integration.yml`

The MySQL integration workflow starts a MySQL 8.4 service, loads
`fixtures/mysql/init/`, and runs:

```sh
script/mysql-integration.sh
```

Generated TypeScript CI follows the same trigger and reusable workflow split:

- Trigger layer:
  `.github/workflows/on_pull_request_typescript-generated-check.yml` and
  `.github/workflows/on_push_typescript-generated-check.yml`
- Reusable Workflow layer: `.github/workflows/_typescript-generated-check.yml`
- Composite Action layer: `.github/actions/setup-node/action.yml`

The generated TypeScript workflow installs npm dependencies from `package-lock.json` and runs:

```sh
npm run typecheck:generated
```

Rust lint policy is defined in `Cargo.toml`. The default is intentionally strict:
Rust warnings are denied, unsafe code is forbidden, and Clippy `all`, `pedantic`,
`nursery`, and `cargo` lint groups are denied. If a lint is too noisy for a specific
case, lower that lint by name instead of weakening a whole lint group. Prefer local
`#[allow(..., reason = "...")]` attributes when a lint only needs an exception in
one place.
