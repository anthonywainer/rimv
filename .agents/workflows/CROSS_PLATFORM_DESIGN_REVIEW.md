# Workflow — Cross-Platform UI Design Review

**Invoke:** a UI flow is added/changed on two or more platforms, a shared token or UX state is changed, or a cross-platform inconsistency is reported.

**Read only what's relevant:** `.aiassistance/design/cross-platform/CONSISTENCY_CONTRACT.md`, affected component/state docs, target platform guides, then QA checklist. Use `.aiassistance/skills/cross-platform-ui/SKILL.md` for step-by-step AI execution.

## Phase 1 — Define actual scope

- Name the feature and user goal.
- Identify currently implemented and affected platform targets; do not assume a platform exists just because its guide does.
- Identify expected states and interaction contracts.
- Locate actual UI files and existing design tokens before proposing a redesign.

## Phase 2 — Establish invariants

Check what must be identical in meaning: capture state truth, source vs AI distinction, error recovery, semantics, preferences and accessible operation. Separate those from permissible native navigation, windowing and input patterns.

## Phase 3 — Make a platform matrix

Use `design/cross-platform/templates/PARITY_REVIEW.md`. Record for each target: support state, current implementation, native interaction, accessibility approach, test environment and evidence.

## Phase 4 — Compare design choices

Look for unwanted drift:

- one platform has missing/renamed essential state;
- buttons use contradictory semantics;
- user is told contradictory retention/privacy information;
- access failure is recoverable on one platform but unexplained on another;
- a native difference is mistaken for a defect.

Record lasting justified deviations using `templates/DESIGN_EXCEPTION.md`.

## Phase 5 — Implement minimally

Change only affected components/adapters. Avoid broad re-skinning of existing platforms as a side effect of one UI fix. Use the Part 4A canonical design system rather than copying its palette into new documents.

## Phase 6 — Validate

- Test the shared meaning and concrete interactions on each *available* target.
- Test an appropriate accessibility mode and scaling scenario.
- Use per-platform visual baselines when available; do not compare pixels between OSes as a parity criterion.
- Explicitly mark all untested/unimplemented targets.

## Phase 7 — Report

Keep the final result compact:

```text
Feature: [name]
Confirmed: [tested platforms / scenarios]
Native adaptations: [important deliberate differences]
Not verified: [platform + reason]
Issues: [actionable gaps only]
```

Do not generate a lengthy cross-platform report unless requested; save a completed template when a lasting decision, handoff, or release gate needs documentation.
