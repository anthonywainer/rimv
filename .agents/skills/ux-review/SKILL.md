---
name: ux-review
description: Review existing RimV flows from provided UI/code/prototype evidence and prepare concise actionable design feedback or release review without inventing participants.
---

# Skill — evidence-based UX review

## Trigger

Use for design review, expert walkthrough, prototype feedback, post-change UX regression and release-readiness questions. For a formal heuristic-only audit, apply the Part 4C.1 skill. For participant studies, apply `ux-testing`.

## Process

1. Identify platform, affected task, expected outcome, provided evidence and product build.
2. Inspect the smallest relevant UI/code/screenshot and canonical design system.
3. Use the matching platform guide and cross-platform contract only when relevant.
4. Walk happy path and relevant error/cancellation/permission path; record what is directly visible or reproducible.
5. Apply relevant heuristics and accessibility references without calling this a full WCAG audit.
6. Describe concrete observed behavior, user impact, severity and smallest proportionate change.
7. For major changes, use `.aiassistance/workflows/UX_RELEASE_REVIEW.md` and flag untested platforms honestly.

## Evidence labeling

- `Observed` — directly supported by actual code, image, test or participant notes.
- `Inferred risk` — plausible but not observed; needs validation.
- `Not tested` — do not mark as pass.

Do not invent interviews, screenshots, usage metrics or user quotes. Avoid redesigning an entire screen because of a localized issue.

## Output

Use a concise findings list ordered by impact, with paths/screen names when known; note the evidence source and concrete retest condition.
