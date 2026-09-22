# RimV Web Platform Guidelines

## Scope

Use these rules for `apps/web`.

The shared RimV design system remains the source of truth for colors, spacing, typography, radii, motion, and semantic states.

## 1. Semantic HTML first

Use semantic HTML before generic containers.

Prefer:

- `button` for actions;
- `a` for navigation;
- `label` for form labels;
- `input`, `select`, `textarea` for form controls;
- `nav` for navigation;
- `main` for primary content;
- `aside` for contextual content;
- headings in logical order.

Do not recreate native semantics with `div` + click handlers unless necessary.

## 2. No framework assumption

The current stack does not declare React, Vue, Svelte, or Angular.

Do not introduce a framework simply to build ordinary UI.

Use:

- TypeScript modules;
- DOM APIs;
- small reusable utilities;
- Tailwind classes;
- lightweight state patterns already present in the project.

## 3. Responsive layout

The layout should adapt by content priority rather than by shrinking desktop UI.

### Large desktop

Possible structure:

```text
navigation
main workspace
context panel
```

### Medium

Collapse secondary panels before reducing core readability.

### Small

Use a single-column flow.

Move secondary content into:

- disclosure sections;
- dialogs;
- drawers;
- separate views.

Do not preserve three-column desktop layouts on narrow screens.

## 4. Breakpoints

Use existing Tailwind breakpoints unless product requirements justify custom ones.

Avoid arbitrary breakpoint proliferation.

Responsive behavior should be based on layout needs rather than device brand names.

## 5. Browser zoom

The UI must remain usable at increased browser zoom.

Avoid:

- fixed-height text containers;
- absolute positioning for core content;
- tiny controls;
- layouts that require horizontal scrolling at common zoom levels.

## 6. Typography

Use the RimV type hierarchy and web-safe/system fallback stack.

Do not use fixed pixel heights that clip enlarged text.

Core readable content should not be visually compressed to fit more information.

## 7. Design tokens

Use semantic design tokens rather than repeated literals.

Preferred semantic concepts:

- background;
- surface;
- surface-secondary;
- border;
- text-primary;
- text-secondary;
- live;
- ai;
- warning;
- error.

Do not hard-code semantic meaning into raw color names throughout the UI.

## 8. Tailwind usage

Prefer:

- reusable class patterns;
- semantic CSS variables;
- shared component utilities;
- consistent spacing/radii.

Avoid:

- extremely long duplicated class lists;
- arbitrary values when a token exists;
- one-off colors not in the design system;
- hidden responsive behavior that is difficult to trace.

## 9. CSS variables

Map RimV design tokens into CSS variables.

Example categories:

```text
--rimv-bg
--rimv-surface
--rimv-text-primary
--rimv-text-secondary
--rimv-live
--rimv-ai
--rimv-warning
--rimv-error
--rimv-radius-md
--rimv-space-4
```

Theme switching should update semantic variables rather than individual component styles.

## 10. Light and dark themes

Respect `prefers-color-scheme` where it matches product behavior.

If RimV provides an explicit theme preference:

- system;
- light;
- dark

is a sensible model.

Do not create separate component markup for each theme.

## 11. Focus

All keyboard-focusable controls must have a visible focus state.

Do not globally remove outlines.

Custom focus styling must be at least as discoverable as the browser default.

## 12. Keyboard

Critical flows must work without a mouse.

Important actions may include:

- start/stop capture;
- navigation;
- device/model selection;
- copy transcript;
- open/close dialogs;
- submit forms;
- dismiss temporary UI.

Use browser-standard keyboard behavior where possible.

## 13. Dialogs

Use native `<dialog>` where suitable, or an accessible dialog implementation.

A dialog must:

- receive focus when opened;
- contain logical tab order;
- close predictably;
- restore focus when dismissed;
- expose an accessible name.

Avoid modal chains.

## 14. Tooltips

Tooltips supplement labels; they should not be the only source of essential information.

Do not hide critical meaning behind hover-only tooltips.

## 15. Buttons

Primary action:
- one dominant action per local context.

Secondary action:
- lower visual emphasis.

Destructive action:
- explicit destructive semantics.

Icon-only buttons:
- require accessible labels;
- need adequate pointer/touch targets.

## 16. Inputs

Each input should have an accessible label.

Placeholder text does not replace a label.

For validation errors:

- identify the field;
- explain the issue;
- keep entered data where practical;
- focus or announce errors appropriately.

## 17. Forms

Avoid unnecessary confirmation steps.

Disable submit only when the reason is understandable.

When submission is in progress:

- show pending state;
- prevent duplicate submission where needed;
- preserve recoverability.

## 18. Loading states

Use:

- skeletons for structured content where useful;
- spinners/progress indicators for isolated actions;
- progress bars for measurable long operations.

Do not leave blank screens during load.

## 19. Recording/transcription states

Explicitly represent:

- idle;
- permission required;
- preparing;
- listening;
- recording;
- transcribing;
- processing;
- complete;
- error.

Never communicate live state through animation or color alone.

## 20. Live transcript behavior

During live updates:

- avoid stealing focus;
- avoid resetting text selection;
- preserve user scroll position where practical;
- avoid jumping to bottom if the user intentionally scrolled upward;
- distinguish partial vs final text if the engine exposes both.

## 21. Long transcripts

Long transcript views should:

- remain selectable;
- support copying;
- avoid excessive DOM churn;
- maintain readable line length;
- preserve user position.

Consider virtualization only when measurement proves it is necessary.

## 22. AI-generated content

AI content should be distinguishable from source transcript content.

Use the RimV `ai` semantic accent sparingly.

Do not make AI output visually dominate the source unless the product flow explicitly prioritizes it.

## 23. Errors

User-facing errors should explain:

1. what happened;
2. what the user can do.

Avoid exposing raw stack traces or transport errors as the primary message.

Technical details may be available separately.

## 24. Empty states

Examples:

- no transcript yet;
- no available model;
- no audio device;
- no history.

Each empty state should guide the next useful action.

## 25. Permission flows

Browser microphone permission is a first-class flow.

Before requesting:

- tie the request to a user action;
- explain why microphone access is needed.

When denied:

- explain what is unavailable;
- provide browser-specific recovery guidance only when appropriate.

Do not repeatedly call the permission request after denial.

## 26. Media/device APIs

When working with browser media APIs:

- handle missing device;
- handle permission denial;
- handle device disconnect;
- handle default device changes;
- avoid retaining streams unnecessarily;
- stop tracks when capture ends.

## 27. Notifications

Use in-app notifications/toasts for transient application feedback.

Reserve browser/system notifications for cases where background attention is actually useful.

Do not ask for notification permission without clear user benefit.

## 28. Motion

Respect:

```css
@media (prefers-reduced-motion: reduce)
```

Reduce decorative movement.

Keep state changes understandable without animation.

## 29. Contrast

Text and meaningful UI must meet accessible contrast requirements.

Do not use opacity-heavy gray text for important information.

## 30. Touch

Although primarily desktop-oriented, web UI may run on touch devices.

Interactive targets should be comfortably tappable.

Do not require hover for essential behavior.

## 31. Pointer states

Controls should define appropriate:

- default;
- hover;
- focus;
- active;
- disabled;
- loading

states.

## 32. Progressive enhancement

Core interaction should not depend on unnecessary advanced browser features.

When optional capabilities are unavailable, degrade gracefully.

## 33. Storage

If using browser storage:

- store only what is needed;
- avoid persisting sensitive transcript/audio data by default;
- document user-visible persistence behavior.

## 34. Performance

Prefer:

- small DOM updates;
- delegated event handling where useful;
- lazy work;
- efficient rendering of long content;
- limited dependencies.

Measure before adding complex optimization.

## 35. Web QA checklist

- [ ] semantic HTML used
- [ ] keyboard flow works
- [ ] visible focus exists
- [ ] browser zoom remains usable
- [ ] responsive layout works
- [ ] light/dark works
- [ ] reduced motion respected
- [ ] color not sole state signal
- [ ] dialogs manage focus correctly
- [ ] microphone-denied flow exists
- [ ] loading/empty/error states exist
- [ ] live transcript does not disrupt user position
- [ ] AI content is clearly distinguished
- [ ] no unnecessary framework introduced
