# Issue Label Taxonomy

This repo uses labels as a lightweight project-management system. Labels should make an issue understandable at a glance: what kind of work it is, what area it touches, and whether it is suitable for a new contributor.

## Label principles

- Prefer a small number of accurate labels over a large pile of vague ones.
- Use labels to clarify scope, not to repeat the title.
- Keep dependency details in the issue body with `Depends on` and `Blocks` sections.
- When an issue changes shape, update the labels and relationship notes together.

## Core work-type labels

| Label | Use when |
| --- | --- |
| `bug` | Existing behavior is broken or incorrect. |
| `feature` | The issue adds new user-facing or workflow-facing capability. |
| `enhancement` | The issue improves existing behavior without creating a major new feature. |
| `documentation` | The primary work is docs, examples, README/wiki cleanup, or contributor guidance. |
| `good first issue` | The issue is scoped, low-risk, and includes enough detail for a new contributor to start. |

## Area labels

| Label | Use when |
| --- | --- |
| `ci` | The issue touches GitHub Actions, release automation, pipeline behavior, artifacts, or CI guardrails. |
| `infra` | The issue affects repository infrastructure, packaging, logging, release setup, or maintainability. |
| `output` | The issue changes stdout/stderr behavior, output formats, file output, logging safety, or machine-readable output. |
| `advanced` | The issue is a larger feature expansion that depends on existing architecture or requires deeper project context. |

## Relationship labels vs relationship sections

Labels describe the type and area of work. They do not replace dependency tracking.

Use the issue body for dependency relationships:

```markdown
## Depends on
- #23 structured logging/output safeguards, because JSON output must remain machine-readable.

## Blocks
- #24 cross-platform release binaries, because release artifacts depend on stable output behavior.
```

When issue A says it blocks issue B, issue B should also say it depends on issue A. Mirrored relationships keep the tracker useful when contributors read issues in isolation.

## Recommended label combinations

| Issue type | Suggested labels |
| --- | --- |
| CLI feature | `feature` plus any area labels such as `output` or `ci` |
| Release workflow | `feature`, `infra`, `ci` |
| Packaging/distribution | `feature`, `infra` |
| Logging/output safety | `enhancement`, `infra`, `output` |
| Advanced parser expansion | `feature`, `advanced` |
| README/wiki/docs cleanup | `documentation` |
| Beginner-friendly docs task | `documentation`, `good first issue` |

## Triage checklist

Before leaving an issue open, confirm:

- [ ] The labels describe the work type and touched area.
- [ ] The issue has clear acceptance criteria.
- [ ] Dependency relationships are mirrored where relevant.
- [ ] The issue has a milestone when it belongs to a known phase.
- [ ] The issue is small enough to close in one focused pull request, or explicitly explains why it is larger.
