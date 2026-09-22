# WCAG 2.2 A + AA — RimV Web Baseline

**Normative source:** https://www.w3.org/TR/WCAG22/
**Human-friendly reference:** https://www.w3.org/WAI/WCAG22/quickref/

RimV Web targets **all applicable Level A and AA success criteria**, not only those highlighted here. This is an implementation checklist, not a claim of audited conformance or legal compliance. Check the normative WCAG text if a criterion, exception, or interpretation is uncertain.

## High-priority criteria for RimV

| Criterion | Level | What to verify in RimV |
|---|---|---|
| 1.1.1 Non-text Content | A | Buttons/icons/graphics have useful text alternatives; decorative visuals are hidden from assistive tech. |
| 1.2.1 Audio-only and Video-only (Prerecorded) | A | Provide equivalent alternatives for prerecorded audio/video **when such content is published**, accounting for its purpose and exceptions. |
| 1.2.2 Captions (Prerecorded) | A | Prerecorded synchronized media has captions where applicable. An automatic transcript may need correction. |
| 1.2.4 Captions (Live) | AA | Provide captions when RimV itself publishes live synchronized media in scope; an app merely transcribing incoming audio is not automatically equivalent. |
| 1.3.1 Info and Relationships | A | Semantic regions, heading order, label/control relationships and transcript structure. |
| 1.3.2 Meaningful Sequence | A | Reading order makes sense with CSS/layout disabled or linearized. |
| 1.3.3 Sensory Characteristics | A | Instructions do not depend solely on location, shape or color. |
| 1.3.4 Orientation | AA | Avoid locking orientation unless essential. |
| 1.3.5 Identify Input Purpose | AA | Use appropriate autocomplete for inputs covered by the criterion. |
| 1.4.1 Use of Color | A | `Recording`, `Paused`, `Error`, and `AI summary` remain identifiable without color. |
| 1.4.3 Contrast (Minimum) | AA | Normal text >= 4.5:1; large text >= 3:1, subject to normative exceptions. |
| 1.4.4 Resize Text | AA | Text can scale to 200% without loss of content/functionality (subject to criterion exceptions). |
| 1.4.5 Images of Text | AA | Use real text when practical; avoid essential labels baked into images. |
| 1.4.10 Reflow | AA | At 320 CSS-px-equivalent width, ordinary vertical content reflows without two-dimensional scrolling, except inherently two-dimensional content. |
| 1.4.11 Non-text Contrast | AA | Required visual control boundaries/states and meaningful graphic parts achieve >= 3:1 against adjacent colors. |
| 1.4.12 Text Spacing | AA | User-specified line/paragraph/letter/word spacing does not clip or obscure content. |
| 1.4.13 Content on Hover or Focus | AA | Custom tooltips/popovers support dismissal, pointer hover and persistence where required. |
| 2.1.1 Keyboard | A | Start/stop capture, model setup, export, dialogs and settings work with keyboard. |
| 2.1.2 No Keyboard Trap | A | Keyboard users can leave all controls and dialogs. |
| 2.1.4 Character Key Shortcuts | A | Single printable-character shortcuts are off/remappable/active only with focus as required. |
| 2.2.1 Timing Adjustable | A | Time-limited actions offer the required options unless an exception applies. |
| 2.2.2 Pause, Stop, Hide | A | Automatically moving/updating content has required controls where applicable. Do not confuse a live recording indicator with purely decorative motion. |
| 2.3.1 Three Flashes or Below Threshold | A | No flashing patterns beyond thresholds. |
| 2.4.1 Bypass Blocks | A | Repeated page chrome can be skipped. |
| 2.4.2 Page Titled | A | Meaningful document title. |
| 2.4.3 Focus Order | A | Tab sequence matches logical workflow. |
| 2.4.4 Link Purpose (In Context) | A | Links describe destinations/actions in context. |
| 2.4.5 Multiple Ways | AA | Relevant pages are reachable by more than one way, subject to process exceptions. |
| 2.4.6 Headings and Labels | AA | Clearly describe content or purpose. |
| 2.4.7 Focus Visible | AA | Keyboard focus indicator never disappears. |
| 2.4.11 Focus Not Obscured (Minimum) | AA | Focused component is not **entirely hidden** behind authored sticky content. Prefer fully visible when feasible. |
| 2.5.1 Pointer Gestures | A | Multipoint/path gestures have single-pointer alternatives unless essential. |
| 2.5.2 Pointer Cancellation | A | Avoid accidental irreversible activation on pointer-down. |
| 2.5.3 Label in Name | A | Accessible names contain visible labels for speech-input users. |
| 2.5.4 Motion Actuation | A | Device-motion actions have UI alternatives and accidental activation can be disabled as required. |
| 2.5.7 Dragging Movements | AA | Provide an equivalent single-pointer alternative for dragging unless essential or user-agent-controlled. |
| 2.5.8 Target Size (Minimum) | AA | Pointer targets normally >= **24 x 24 CSS px**, or meet a normative exception such as sufficient spacing. RimV's preferred touch target can be larger. |
| 3.1.1 Language of Page | A | Declare page language. |
| 3.1.2 Language of Parts | AA | Mark known language changes in content where applicable. |
| 3.2.1 On Focus | A | Focusing a control does not unexpectedly change context. |
| 3.2.2 On Input | A | Changing a control doesn't unexpectedly change context without notice. |
| 3.2.3 Consistent Navigation | AA | Repeated navigation keeps order. |
| 3.2.4 Consistent Identification | AA | Same-function controls have consistent labels and semantics. |
| 3.2.6 Consistent Help | A | Repeated help mechanisms occur in consistent order when present. |
| 3.3.1 Error Identification | A | Errors identify the relevant item and describe the problem in text. |
| 3.3.2 Labels or Instructions | A | Inputs have labels/instructions when needed. |
| 3.3.3 Error Suggestion | AA | Offer corrective suggestions when known and safe. |
| 3.3.4 Error Prevention (Legal, Financial, Data) | AA | Apply when a workflow meets the specified consequence conditions. |
| 3.3.7 Redundant Entry | A | Avoid unnecessarily requiring repeated data in one process, with exceptions. |
| 3.3.8 Accessible Authentication (Minimum) | AA | If authentication is added, ensure compliant alternatives to cognitive function tests; don't require memorization/puzzles alone. |
| 4.1.2 Name, Role, Value | A | Custom controls expose role, name, state and changes. |
| 4.1.3 Status Messages | AA | Important status updates are programmatically determinable without moving focus. |

## Important WCAG 2.2 distinctions

- `2.4.11 Focus Not Obscured (Minimum)` is **AA**; `2.4.12 Enhanced` is **AAA**.
- `2.4.13 Focus Appearance` is **AAA**, not AA. RimV can still use a stronger focus indicator as good practice.
- `2.5.8 Target Size (Minimum)` is **24 x 24 CSS px** at AA, with documented exceptions. `2.5.5 Target Size (Enhanced)` is **44 x 44 CSS px** at AAA.
- WCAG 2.2 removed former criterion `4.1.1 Parsing`; do not add it as a 2.2 requirement.
- A green dot plus a textual `Recording` indicator satisfies the color-only concern more effectively than changing the dot color alone; it does **not** by itself establish conformance with all other criteria.

## RimV acceptance examples

- User can operate capture with a keyboard, including a reachable and named Stop button.
- Permission denial preserves task context and provides an accessible way to open settings/retry.
- Live transcript updates do not repeatedly replace the whole accessible tree or move focus.
- Long transcripts reflow and retain logical reading order.
- Model-download progress is readable as a percentage when known; errors offer recovery.
- Theme token combinations are evaluated **at actual point of use**, not assumed compliant because they are in the design system.

## Verification order

1. Automated check for parseable markup, missing names and simple contrast failures.
2. Keyboard-only walk of all affected paths.
3. Screen-reader interaction with actual focus order and dynamic states.
4. Zoom, text spacing, dark/light and forced colors.
5. Manual judgment against every applicable A and AA criterion before making any conformance claim.
