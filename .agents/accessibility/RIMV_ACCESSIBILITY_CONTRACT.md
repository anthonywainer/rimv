# RimV Accessibility Contract

A compact set of cross-platform behaviors. This is the shared functional contract; platform documents explain implementation differences.

## Core user outcomes

1. **Start:** discover and activate recording/listening through an accessible primary control.
2. **Know:** perceive current capture state through readable text and accessible state, not only color, waveform or sound.
3. **Stop:** reach and activate a clearly named Stop control throughout live capture.
4. **Read:** select, navigate, copy and export a transcript without continuous focus jumps.
5. **Recover:** permission, device, model and inference failures have actionable guidance.
6. **Control:** cancel long tasks where technically safe, choose input device and undo/retry where applicable.
7. **Adapt:** use keyboard, pointer/touch, screen reader, zoom/text scaling, high contrast and reduced motion.

## Accessibility ownership

- **Domain/engine:** emits truthful states and structured errors, not accessibility labels.
- **Platform adapter:** maps engine state into the platform's accessible status, progress, focus and control semantics.
- **UI component:** owns a clear name, role, value/state, hit target and focus behavior.
- **Content layer:** preserves transcript text, partial/final distinctions and logical order.
- **QA:** tests real flows and records platform/device/tool versions and untested cases.

## Core states

`idle → permission-required → preparing → listening/recording ↔ paused → stopping → transcribing/finalizing → completed`

Alternative exits: `permission-denied`, `device-unavailable`, `model-unavailable`, `cancelled`, `error`.

Not all apps use every state. Do not invent an engine state to make the UI flow look complete. UI must accurately reflect real engine behavior.

## Non-negotiable controls

- Live capture: discoverable Stop; Pause only if engine supports pause; microphone/device if changeable.
- Long operation: state + progress if measurable; cancel when safe, otherwise say cancellation isn't possible.
- Error: descriptive message, recovery action, optional technical details.
- Dialog: logical focus entry/exit; descriptive title; no surprise state loss.
- Transcript: text is selectable/reachable; updates do not steal focus; streaming output is not incessantly announced.

## Privacy and agency

Do not auto-speak sensitive transcript content through a screen reader's live region or system notification. Important status can be announced; transcript content is accessed on demand unless the user explicitly enables read-aloud updates. Do not claim recording stopped until the actual capture operation has stopped.

## Design system relationship

The canonical tokens and component specs live in `.aiassistance/design/`. Accessibility may require an *accessible use* of a different semantic token, stronger outline or platform system color. Do not silently rewrite Part 4A; document a contrast issue and add an explicit accessible token mapping when approved.
