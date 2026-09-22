---
name: cross-platform-ui
description: "Use for RimV UI work affecting multiple of macOS, iOS, Windows, Linux or Web; shared design tokens, recording/transcript state parity, adaptive layouts and multi-platform UX reviews. Not for isolated backend portability or one-platform cosmetic fixes."
---

# Cross-Platform UI Consistency

## Trigger

Invoke when a UI/UX change spans **two or more platforms** or changes the meaning of shared visual/state tokens. For a single-platform UI fix, use that platform's own UI skill and consult this skill only if a shared invariant might be affected. For Rust portability, use `.aiassistance/skills/cross-platform/SKILL.md` instead.

## Required minimal reading

1. `.aiassistance/design/cross-platform/CONSISTENCY_CONTRACT.md` (shared invariants).
2. Relevant section of `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`; token JSON only if changing token mappings.
3. One or more target-platform guides: `platforms/apple/`, `windows/`, `linux/`, `web/`.
4. Relevant cross-platform detail only: `STATE_AND_COPY_CONTRACT.md`, `COMPONENT_PARITY.md`, `ADAPTIVE_LAYOUTS.md` or `TOKEN_ADAPTERS.md`.
5. `CROSS_PLATFORM_QA.md` for validation; use the full review workflow for substantial changes.

**Do not load every guide on every task.**

## Procedure

1. **Scope:** identify actual implemented targets; don't infer support from directory names or documentation.
2. **User goal:** define the shared behavior/observable states.
3. **Map:** locate actual UI code and existing design token adapters in the affected apps.
4. **Separate:** list semantic invariants vs native presentation differences.
5. **Implement:** keep token values canonical and adapt through platform-specific resource/theme layers; never copy a web component layout verbatim to native UI.
6. **Truthful state:** confirm Start/Stop transitions with actual runtime events; distinguish partial/final transcript and AI output.
7. **Accessibility:** check keyboard/touch, labels, contrast, font scaling, focus and reduced motion for each target.
8. **Validate:** use the QA matrix; explicitly mark untested/not implemented/unsupported targets rather than assuming parity.
9. **Escalate:** cross-platform contract changes → Software Architect; privacy or microphone claims → Security Engineer; fundamental UX changes → UI/UX Designer.

## Deliverable

For small tasks: a concise status listing tested targets and gaps. For a release-sensitive change: fill `design/cross-platform/templates/PARITY_REVIEW.md`. Use `templates/DESIGN_EXCEPTION.md` only for deliberate lasting platform differences. Do not write broad reports by default.
