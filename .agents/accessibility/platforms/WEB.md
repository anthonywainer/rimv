# Web — Accessibility Implementation and Test Guide

**Applies to:** `apps/web`, currently TypeScript + Vite 6 + Tailwind CSS 3 + native Node tests. Do not assume React or Vue.

## Markup and DOM

- Prefer semantic HTML (`main`, `nav`, `button`, `label`, `form`, `dialog`, headings, real tables for tabular data).
- Use real `button` elements for capture, Stop, device selection, copy and model commands. Use anchors for navigation.
- Add ARIA when native semantics are insufficient; avoid replacing valid native semantics with custom ARIA roles.
- Use accessible names containing the visible button text. Icon-only controls need a name.
- Set page `lang` and mark known mixed-language passages when feasible; transcription language may be uncertain before detection.
- Avoid injecting transcript content with unsanitized `innerHTML`; keep user speech treated as untrusted text.

## Recording UI

```html
<div aria-labelledby="recording-heading">
  <h1 id="recording-heading">Audio capture</h1>
  <p id="recording-status" role="status" aria-atomic="true">Ready to listen</p>
  <button type="button" id="start-listening">Start listening</button>
  <button type="button" id="stop-listening" hidden>Stop recording</button>
</div>
```

This is illustrative; wiring must reflect *actual* engine state. When switching buttons, prevent focus loss if a focused button disappears; a persistent Start/Stop control with an updated label may be simpler.

## Transcript

- Keep a labeled transcript region and stable reading order.
- Treat the streaming transcript as `aria-live="off"` by default; a separate concise `role="status"` region can announce `Transcript ready` or `Recording started`.
- Avoid `role="alert"` for non-urgent events and avoid every-word announcements.
- Preserve selection and scroll position when new text arrives; offer `Jump to latest` if the user has scrolled away.
- Distinguish partial/final content visibly and programmatically where useful.

## Dialogs and keyboard

- Prefer native `<dialog>` where its behavior fits; otherwise implement equivalent labeling, focus entry/containment/return and Escape.
- Test Tab/Shift+Tab, Enter/Space, dialog close, browser zoom and browser/device permission flows.
- Ensure sticky toolbars never entirely hide focused controls at narrow viewport sizes.

## CSS and motion

- Map semantics to the Part 4A token system; check rendered contrast for actual text/background pairs.
- Test forced colors and `prefers-reduced-motion`.
- At 200% and 400% zoom, verify reflow and usable Stop control; add breakpoints only when needed for the layout.
- Verify custom text-spacing overrides and 200% text size, not merely viewport width.

## Test strategy

1. `npm run typecheck` / `npm run test` / `npm run build` for engineering regressions; **these do not prove accessibility**.
2. Run a web accessibility analyzer where available; treat findings as leads rather than full compliance proof.
3. Keyboard-only end-to-end flow.
4. Test at least one browser+screen-reader combination appropriate to the target audience. Where possible test NVDA/Firefox or Chrome on Windows and VoiceOver/Safari on macOS/iOS.
5. Manual light/dark, forced colors, zoom, permissions, long transcript, model download and failure recovery.

## Acceptance

A keyboard/screen-reader user can start capture, know the actual state, stop, review the final transcript and recover from an error without being forced into a streaming announcement flood.
