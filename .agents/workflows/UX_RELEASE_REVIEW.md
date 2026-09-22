# Workflow — UX release review

**Scope:** Apply to significant capture, transcription, permission, model, export or cross-platform UI changes; scale down for trivial styling edits.

1. Identify exact release build, affected platforms, user goal and design/API changes.
2. Collect Part 4C.1 usability issues, Part 4C.2 accessibility results and available actual user-study evidence; do not assume any missing evidence passed.
3. Run targeted happy-path and error-path regression for each affected platform that is testable.
4. Check `ux/testing/UX_RELEASE_GATE.md`: recording stop/state, permissions, device loss, model progress, partial/final transcript, copy/export, privacy language.
5. Compare platform behavior to `design/cross-platform/CONSISTENCY_CONTRACT.md` and record intentional native differences.
6. Triage unresolved issues and document owner, mitigation, retest and release decision. Escalate critical/major problems explicitly.
7. State `PASS`, `PASS WITH KNOWN ISSUES`, `BLOCKED`, or `NOT EVALUATED` for **this scope only** and attach what was actually tested.

Do not imply that an expert review or a few qualitative tests guarantee product-wide usability or WCAG conformance.
