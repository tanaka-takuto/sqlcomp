# GitHub Actions Layout

This repository keeps GitHub Actions in three layers so CI changes stay small and
predictable. The layers separate when CI runs from what CI does and how shared setup
is performed.

## Layers

### Trigger Workflows

Trigger workflows are entry points. They define GitHub events, branch filters, permissions, and
inputs passed to reusable workflows.

Keep trigger workflows thin:

- Do not add formatting, build, test, or deploy steps here.
- Do not duplicate job logic across trigger workflows.
- Name files as `on_<event>_<check-family>.yml`.

### Reusable Workflows

Reusable workflows own CI orchestration. They define jobs, runners, shared permissions, and the
sequence of repository-level checks.

Use this layer when changing what a CI job does, such as adding a check command or
changing the order of checks.

Reusable workflow filenames are named `_<check-family>.yml` and are invoked through
`workflow_call`.

### Composite Actions

Composite actions provide reusable step groups. They are for low-level setup or repeated command
sequences, not for deciding when CI runs.

Use this layer when changing how a tool is installed or initialized.

Composite actions are named `setup-<tool>/action.yml` when their main purpose is
tool setup.

## Maintenance Guide

When adding or changing CI, first decide which layer owns the change:

- New event or branch/path filter: add or edit a trigger workflow.
- New job behavior or check orchestration: add or edit a reusable workflow.
- Repeated setup steps or tool installation: add or edit a composite action.
