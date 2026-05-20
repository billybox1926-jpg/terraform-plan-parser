# Wiki Alignment Guide

The GitHub Wiki should act as a lightweight operations handbook for contributors and maintainers. It should not duplicate the detailed CLI reference, architecture notes, roadmap, or maintainer workflow already tracked in the repository.

## Canonical documentation surfaces

- `README.md`: public landing page, installation, quickstart, CLI usage, examples, and configuration reference.
- `docs/ARCHITECTURE.md`: technical design, data flow, and implementation decisions.
- `docs/CONTRIBUTING.md`: contributor setup, local checks, and collaboration expectations.
- `docs/ROADMAP.md`: completed, current, next, and later project direction.
- `docs/ISSUE_LABELS.md`: label taxonomy and triage checklist.
- `docs/MAINTAINER_WORKFLOW.md`: issue lifecycle, dependency mirroring, milestones, PR review, and closing standards.
- GitHub Issues: active scoped work, acceptance criteria, dependencies, labels, and milestones.
- GitHub Wiki: project operations index, contributor navigation, maintainer navigation, triage guidance, PR review guidance, and release-operation links.

## Wiki page guidance

The wiki should link to canonical repository docs instead of copying large sections from them. This keeps the wiki useful without letting it drift into a second README.

Recommended wiki pages:

- `Home.md`: project operations index with links to README, docs, issue tracker, and release-readiness issues.
- `Developer & Usage Guide`: short contributor orientation that links back to README for CLI usage and `docs/ARCHITECTURE.md` for implementation details.

## Content boundaries

Do not duplicate detailed CLI flags, configuration keys, architecture diagrams, or full maintainer workflow rules in the wiki. Keep those details in the repository docs and use the wiki to help people find the right canonical source.

## Maintenance checklist

When repository docs change, review the wiki for stale links or duplicated content. When issues, labels, milestones, or release processes change, link to the canonical issue or docs page instead of rewriting the same information in the wiki.
