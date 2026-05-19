# Maintainer Workflow

This repo treats issues as the project-management layer, not just a place to park ideas. A good issue should explain the work, the reason it matters, the labels that describe it, the dependencies around it, and the conditions required to close it.

## Canonical project surfaces

Use each project surface for a clear purpose:

| Surface | Purpose |
| --- | --- |
| `README.md` | Root landing page, user quickstart, installation, CLI usage, and examples. |
| `docs/ARCHITECTURE.md` | Technical design, data flow, and implementation decisions. |
| `docs/CONTRIBUTING.md` | Contributor setup, checks, and collaboration expectations. |
| `docs/ROADMAP.md` | Broad project direction and completed/planned capability areas. |
| `docs/ISSUE_LABELS.md` | Label taxonomy and triage checklist. |
| `docs/MAINTAINER_WORKFLOW.md` | Issue lifecycle, dependency mirroring, milestones, PR review, and closing standards. |
| GitHub Issues | Active work tracking, scoping, acceptance criteria, and dependencies. |
| GitHub Wiki | Lightweight operations handbook and contributor/maintainer navigation. |

Avoid duplicating detailed CLI reference content across surfaces. Link to the canonical document instead.

## Issue lifecycle

### 1. Intake

When opening or reviewing a new issue, make sure it has:

- A clear title using a conventional prefix where helpful, such as `feat:`, `docs:`, `bug:`, `ci:`, or `chore:`.
- A short summary or problem statement.
- Requirements or proposed approach.
- Acceptance criteria written as checkboxes.
- Labels that describe both work type and affected area.
- A milestone when the issue belongs to a known phase.

### 2. Triage

During triage, ask:

- Is this issue actionable without reading the maintainer's mind?
- Is it small enough for one focused pull request?
- Does it duplicate an existing issue?
- Does it depend on other work?
- Does it block other work?
- Is it suitable for `good first issue`, or does it require deeper repo context?

If an issue is too broad, split it into smaller issues and link the relationship in each body.

### 3. Relationship tracking

Use explicit relationship sections whenever an issue depends on or unlocks other work:

```markdown
## Depends on
- #23 structured logging/output safeguards, because machine-readable output must stay clean.

## Blocks
- #24 cross-platform release binaries, because release users need stable output behavior.
```

Relationship notes should be mirrored. If issue A says it blocks issue B, issue B should say it depends on issue A. This makes the dependency graph understandable even when someone lands on one issue directly.

### 4. Labeling

Use `docs/ISSUE_LABELS.md` as the source of truth for label meaning. Labels should answer:

- What kind of work is this?
- What area does it touch?
- How difficult or contributor-friendly is it?

Avoid using labels as a substitute for real scope. The issue body should still explain the work clearly.

### 5. Milestones

Milestones should represent project phases or release-readiness gates, not random buckets. A milestone should make the roadmap easier to scan.

Recommended milestone themes:

- Output safety and CI readiness
- Documentation readiness
- Release readiness
- Distribution readiness
- Advanced parser expansion
- Contributor workflow polish

When an issue moves to a milestone, confirm its dependencies are also assigned appropriately or intentionally left outside the milestone.

### 6. Pull requests

A good pull request should:

- Link the issue it closes or advances.
- Summarize the user-visible change.
- List tests or checks run locally.
- Call out documentation updates.
- Note any follow-up issues instead of hiding unfinished work.

Prefer one focused PR per issue. If a PR intentionally touches multiple issues, explain the relationship in the PR body.

### 7. Closing issues

Before closing an issue, confirm:

- Acceptance criteria are complete or explicitly no longer needed.
- Tests and docs were updated where appropriate.
- Downstream blocked issues have been reviewed.
- Any follow-up work is captured in a new or existing issue.
- The close reason matches reality: completed, duplicate, or not planned.

## Maintainer quality bar

The goal is for a new contributor to open the issue tracker and see a managed project, not a pile of notes. Clear labels, mirrored dependencies, acceptance criteria, and milestone discipline are part of the product experience.
