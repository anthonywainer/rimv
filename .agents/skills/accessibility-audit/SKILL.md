---
name: accessibility-audit
description: "Deep audit of RimV UI accessibility, including WCAG 2.2 AA web requirements and native VoiceOver, Narrator, Orca, keyboard, focus, contrast and live transcription behavior. Use only for implementation or review work that requires detailed accessibility validation."
---

# Accessibility Audit — Deep Skill

This skill **extends** the existing Part 3 `.aiassistance/skills/accessibility/SKILL.md`; it does not replace that quick-use skill.

## Trigger

Use for an explicit accessibility audit, a significant new interactive flow, live transcript/screen-reader bug, keyboard/focus failure, contrast or scaling review, or release-blocking accessibility issue.

Do not load the whole accessibility directory for ordinary Rust/audio implementation with no user interaction impact.

## Select references

- Core: `.aiassistance/accessibility/RIMV_ACCESSIBILITY_CONTRACT.md`
- Web conformance mapping: `WCAG_2_2_AA_BASELINE.md`
- Live/recording changes: `RECORDING_TRANSCRIPTION_ANNOUNCEMENTS.md`
- Focus and input: `KEYBOARD_FOCUS_AND_INPUT.md`
- Contrast/zoom/motion: `VISUAL_AND_MOTION.md` and `DESIGN_TOKEN_CONTRAST_AUDIT.md`
- Target platform only: `platforms/WEB.md`, `APPLE.md`, `WINDOWS.md` or `LINUX.md`
- Evidence plan: `TEST_MATRIX.md`

Read only files relevant to the issue; preserve Part 4A as visual source of truth.

## Procedure

1. Define platform, real user task and failure/success criterion.
2. Inspect actual UI code or reproduce. Do not infer live semantics from static screenshots.
3. Identify input, reading order, state announcements, errors and recovery.
4. For Web, map to applicable WCAG A/AA criteria; distinguish normative requirements from RimV preferences (e.g. 44pt touch goals).
5. Check affected interactions with keyboard/touch, screen reader, focus, theme and scaling.
6. Record findings with concrete evidence and accessible impact; do not fabricate tools/tests not run.
7. Propose smallest compatible change; avoid introducing new frameworks or design-token duplication.
8. Implement if requested, then rerun targeted checks and manual behavior.
9. Record unverified platforms explicitly.

## Critical RimV invariants

- While capture is live, Stop is discoverable without pointer.
- Visible and accessible recording state describe the **actual** engine state.
- Transcript streaming does not steal focus or overwhelm speech output.
- Permission and device errors include an accessible recovery action.
- Sensitive transcript content isn't automatically spoken in system notifications by default.

## Completion

Report only observed issues, impact, affected platform, proposed/implemented remedy and tested or untested scope. Never claim WCAG conformance from a linter or theoretical checklist alone.
