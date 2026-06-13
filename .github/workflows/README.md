# GitHub Actions Layout

This repository keeps GitHub Actions in three layers so CI changes stay small and predictable.

Background: https://zenn.dev/jirtosterone/articles/99cea8c1af0657

## Layers

### Trigger workflows

Files:

- `on_pull_request_format-check.yml`
- `on_push_format-check.yml`

Trigger workflows are entry points. They define GitHub events, branch filters, permissions, and
inputs passed to reusable workflows.

Keep trigger workflows thin:

- Do not add formatting, build, test, or deploy steps here.
- Do not duplicate job logic across trigger workflows.
- Prefer a filename that starts with the triggering event, such as `on_push_` or
  `on_pull_request_`.

### Reusable workflows

Files:

- `_format-check.yml`

Reusable workflows own CI orchestration. They define jobs, runners, shared permissions, and the
sequence of repository-level checks.

Use this layer when changing what a CI job does. For example, change `_format-check.yml` when
adding another formatting check or changing the dprint CI job.

Reusable workflow filenames start with `_` and are invoked through `workflow_call`.

### Composite actions

Files:

- `../actions/setup-dprint/action.yml`

Composite actions provide reusable step groups. They are for low-level setup or repeated command
sequences, not for deciding when CI runs.

Use this layer when changing how a tool is installed or initialized. For example, change
`setup-dprint` when changing the dprint install method.

## Maintenance Guide

When adding or changing CI, first decide which layer owns the change:

- New event or branch/path filter: add or edit a trigger workflow.
- New job behavior or check orchestration: add or edit a reusable workflow.
- Repeated setup steps or tool installation: add or edit a composite action.

## Validation

After editing GitHub Actions files, run these checks locally:

- `dprint fmt`
- `dprint check`
