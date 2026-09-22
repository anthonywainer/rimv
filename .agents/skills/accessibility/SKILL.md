---
name: accessibility
description: "Use when implementing or reviewing interactive RimV UI for keyboard, screen-reader, contrast, touch targets, text scaling, live transcript announcements and reduced motion across platforms."
---

# Accessible Interfaces

## Goal
Enable users to complete RimV's core tasks with keyboard, assistive technology, adjusted text sizes, high-contrast settings and reduced motion.

## Baseline
Use WCAG 2.2 AA as a web-oriented baseline, combined with applicable native platform accessibility guidance. This is an implementation target, not an automatic certification claim.

## Implementation workflow
1. Identify the primary flow (start/stop listening, download model, read transcript, change settings, recover from failure).
2. Inspect actual semantic structure, keyboard order, visible focus and control names.
3. Ensure recording and model status is conveyed by text/accessible state, never only a green/red indicator or animation.
4. Choose accessible status announcement behavior; live transcription must not continuously interrupt a screen reader or steal focus.
5. Check contrast in both themes, including disabled/placeholder states and focus indicators.
6. Check scaling/zoom and touch targets; prioritize readable transcription content.
7. Honor system reduced motion and high-contrast preferences; verify native accessibility APIs are set correctly.
8. Test actual flows with keyboard and at least one relevant screen reader when possible.

## Web checks
Use semantic `<button>`, `<label>`, `<main>`, headings and dialog semantics; prefer native controls. Add ARIA only to supply missing semantics/state. For live regions, announce meaningful final status rather than every high-frequency waveform/timestamp update. `npm run typecheck` does **not** validate accessibility.

## Apple/Windows/Linux checks
Use native accessible labels/roles and system focus conventions; verify VoiceOver/Narrator or desktop screen-reader support as relevant. Respect Dynamic Type/font scaling and system focus behavior. Test the denied-microphone path without mouse-only recovery.

## RimV-specific flows
- Recording must have an unambiguous accessible active-state indicator and Stop action.
- Transcript partial updates must not cause focus jumps.
- Long model downloads need progress/status and cancellation semantics.
- Error messages explain recovery without color-only cues.

## Exit
Core task is keyboard-operable; status and errors are perceivable; content survives scaling; reduced motion is honored; actual screen-reader test scope is reported.
