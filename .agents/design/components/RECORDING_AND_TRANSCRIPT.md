# RimV Recording, Transcript and AI Visual Patterns

This document covers **visual communication**, not full usability audit procedures. The latter will be added in Part 4C.

## Recording status

| State | Visual treatment | Required copy/action |
|---|---|---|
| Idle | Neutral microphone, no pulse | `Start listening` or `Start recording` |
| Requesting permission | Neutral/pending status | `Allow microphone access` + next step |
| Preparing/loading | Neutral spinner with text | `Preparing microphone` or `Loading model` |
| Live | Green dot + restrained pulse or waveform | `Recording`/`Listening` + persistent `Stop` |
| Paused | Explicit pause symbol and neutral/amber styling | `Paused` + `Resume`/`Stop` |
| Stopping/finalizing | Busy indicator; no suggestion that live capture persists | `Finalizing transcript` |
| Completed | Neutral/success summary | `Recording finished` + view/export if applicable |
| Error | Red icon/text and recovery path | Clear explanation + `Retry`/settings action |

Never express recording state using glow, color, waveform animation or sound *alone*. Actual capture status is authoritative, not merely button-toggle state.

## Status chip

- Height: 26px; full radius; 10–12px horizontal padding.
- Use an 8px status dot (or platform-equivalent) and short text.
- `live` green only when capture is actually active.
- `ai` purple only to distinguish generated information; it does not imply confidence.

## Waveform

- May animate while capture is active.
- Must stop animating when capture stops/pauses.
- If animation unavailable or reduced-motion enabled, status text and controls remain fully functional.
- Do not substitute decorative synthetic waveforms for a claim about actual measured audio levels.

## Transcript cards

- Body typography: 16/24 desktop baseline, scalable.
- Text must be selectable and copyable where the platform permits.
- Speaker and timestamp are supporting metadata; make them readable, not faded below meaningful-text contrast.
- `Partial` text is explicitly provisional; `Final` is distinguishable by a stable status label or section treatment.
- Preserve user scroll position; do not forcibly jump on every partial update.
- Long transcripts should not cause other controls to disappear.

## AI cards

- Label generated output as `AI summary`, `AI insight`, etc.
- Purple visual marker helps recognition, but the label remains required.
- Keep generated content separate from source transcript; allow inspection of underlying transcript when relevant.
- Warning or uncertainty uses plain copy, not a decorative confidence percentage without a reliable basis.

## Empty and interrupted states

- No input: `No audio yet` + primary available action.
- Device missing: `Microphone unavailable` + selection/settings link.
- Model unavailable: show the missing model and explicit download/selection action.
- Recoverable engine error: preserve existing transcript; explain retry consequences.
