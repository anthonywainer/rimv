# Surfaces, Cards, Panels and Overlays

## Surface hierarchy

| Depth | Token | Intended use |
|---|---|---|
| 0 | `background` | Full application shell |
| 1 | `surface` | Primary workspace, sidebar, dialog body |
| 2 | `surface-secondary` | Input groups, transcript cards, secondary panels |
| 3 | `surface-tertiary` | Selected or locally raised surfaces; not a new global background |

Use 1px borders to clarify boundaries. Dark-mode surfaces rely on contrast and border before shadows.

## Geometry

- Card: 16px radius; 16–20px padding; 12–16px between cards.
- Panel: 20px radius when appropriate; 24px padding; 20–24px between major desktop panels.
- Internal title/body gap: 10–12px.
- Keep equal edge alignment across adjacent sections.

## Elevation

- Light card: optional `0 1px 2px rgba(15,23,42,.06), 0 6px 18px rgba(15,23,42,.06)`.
- Light modal: `0 8px 32px rgba(15,23,42,.14)` where the dialog needs separation.
- Dark card: ordinarily no shadow; use `border` and different surface levels.
- Dark overlay: restrained shadow only if required to separate a floating surface.

Avoid stacking glows and shadows to simulate hierarchy; prioritize type and spacing.

## Card variants

- Transcript: speaker/source + time + partial/final state + selectable text.
- AI summary: purple `ai` marker + explicit generated-content label.
- Decision/action: semantic success/live color only when action meaning justifies it.
- Warning: warning tint + icon + what the user should do next.
- Error: error tint + specific problem + recovery action.
- Model: file/size/status/action if available; clear download/retry states.

## Modal and sheet behavior

- Appropriate only for important decisions or self-contained tasks.
- Keep focus within a modal while open; restore it on close where applicable.
- State why confirmation is needed; do not demand it for every safe action.
- Avoid obscuring an active session's Stop control with a nonessential modal.

## Empty/loading/error variants

A blank card is not a completed UI state. Show a status statement and, when relevant, an action such as `Import audio`, `Choose microphone`, or `Retry`. Reserve full-screen errors for unrecoverable situations.
