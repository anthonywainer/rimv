# Workflow — Implement Accessible RimV UI

1. **Scope:** identify affected flow, platform, engine state and relevant earlier UI design guidance.
2. **Map states:** enumerate idle, preparing, live, paused if supported, stopping, done, denied, device missing and recoverable failure.
3. **Choose controls:** native semantics first; accessible name, role, value/state; larger targets for important actions.
4. **Plan dynamic announcements:** announce important state transitions sparingly; never automatically stream every transcript token.
5. **Plan focus:** start/stop transition, modal entry/exit, navigation, errors, async completion; preserve user selection.
6. **Apply tokens:** use Part 4A semantics; verify rendered contrast in both themes and high-contrast mode.
7. **Scaling:** verify text scaling, reflow, resizing and reduced motion.
8. **Validate:** focused tests first, then real keyboard/screen-reader check when available; use `.aiassistance/accessibility/TEST_MATRIX.md`.
9. **Document gaps:** if Windows, Linux, iOS or screen-reader testing isn't available, mark it unverified rather than passed.

For browser changes run relevant existing `npm run typecheck`/`npm run test`/`npm run build` as engineering checks only; they do not replace accessibility testing.
