# Keyboard, Focus and Alternative Input

## Desktop keyboard contract

| Action | Keyboard expectation |
|---|---|
| Navigate controls | Tab / Shift+Tab follows logical focus order |
| Activate native button | Enter and/or Space per platform/native control |
| Navigate native list/selection | Platform-standard arrows/home/end when supported |
| Dismiss non-destructive overlay | Escape where the platform convention expects it |
| Start/stop capture | Discoverable named control; shortcut optional and not conflicting |
| Copy/export transcript | Reachable without pointer; platform shortcut optional |
| Dismiss error and recover | Recovery action reachable by keyboard |

Do not create shortcut-only access to an essential control. Do not trap focus in a sidebar, modal or transcript region. Restore focus logically after a modal closes or a transient control disappears.

## Focus management

- Do not move focus when a transcript appends text, a waveform updates or a timer ticks.
- For full-screen navigation, move focus to a logical heading or main region if framework/platform conventions warrant it.
- For modal dialogs, use native focus management or an accessible equivalent; restore to invoking control on close when it still exists.
- A disabled focused button can disappear from the tab order. Before disabling/removing it, ensure the user has an obvious and reachable successor (for example Stop after Start).
- A sticky toolbar must not wholly cover a keyboard-focused element (WCAG 2.4.11 AA on web).

## Pointer/touch

- WCAG 2.2 AA web pointer target: 24×24 CSS px unless a normative exception applies; RimV's larger mobile target recommendation remains valid.
- Offer alternatives to drag-only operations when required (2.5.7 AA).
- Don't trigger destructive changes on pointer down when cancellation is expected (2.5.2 A).
- Visible and programmatic labels should agree for speech-input users (2.5.3 A).
- Core mobile flows should not depend on hard-to-discover gestures alone.

## Test sequence

1. Unplug/avoid mouse, reload/open affected screen.
2. Tab through all controls; note keyboard focus and accessible control names.
3. Start recording; locate Stop without pointer.
4. Trigger a recoverable error; find Retry/Settings.
5. Open/close a dialog; ensure focus returns sensibly.
6. Navigate, select and copy a long transcript.
7. Increase scale/zoom and repeat; check no focused item is hidden.
8. Test screen reader navigation without accidentally announcing partial transcript content.

## Anti-patterns

- `div onclick` for core commands.
- Disabling native outlines with no replacement.
- Focus trap in non-modal popover.
- Automatically focusing every incoming transcript chunk.
- Global single-letter shortcuts that trigger during text entry.
- Small adjacent destructive and confirm targets.
