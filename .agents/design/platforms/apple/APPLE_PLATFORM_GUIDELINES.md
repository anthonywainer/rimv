# RimV Apple Platform Guidelines

## Scope

Use these guidelines for macOS and iOS RimV interfaces.

The shared RimV design system remains the visual source of truth. These rules explain how that system should adapt to Apple platform conventions.

## 1. Native-first interaction

Prefer native SwiftUI controls and platform interaction patterns before creating custom controls.

Use custom controls only when:

- RimV requires a behavior unavailable in standard controls;
- the shared product identity cannot be expressed otherwise;
- the custom control remains accessible;
- the custom control behaves predictably with system settings.

Standard Apple controls already carry platform behavior, keyboard/touch interaction, accessibility semantics, and system appearance support.

## 2. SwiftUI first, not SwiftUI only

Use SwiftUI by default for new UI where it provides the required behavior.

Use AppKit on macOS or UIKit on iOS when:

- a platform API is not adequately exposed through SwiftUI;
- window/menu/status behavior requires native APIs;
- interoperability with existing native views is necessary;
- performance or focus behavior requires lower-level control.

When bridging:

- keep the bridge small;
- preserve accessibility;
- keep platform-specific code localized.

## 3. Platform adaptation

Do not make macOS and iOS use identical layouts.

### macOS

Favor:

- resizable windows;
- sidebars where navigation depth justifies them;
- toolbars for frequent window-level actions;
- menus and keyboard shortcuts;
- context menus where appropriate;
- drag and drop for desktop workflows;
- multi-window support only when it provides clear value.

### iOS

Favor:

- touch-first navigation;
- navigation stacks;
- tabs when top-level sections justify them;
- sheets for focused secondary flows;
- swipe actions where users expect them;
- direct manipulation;
- safe-area-aware layouts.

## 4. RimV live-state semantics

RimV's most important state is often audio/transcription state.

Apple implementations must make these states explicit:

- idle;
- permission required;
- initializing;
- listening;
- recording;
- transcribing;
- processing;
- complete;
- recoverable error;
- unavailable.

Do not communicate recording solely through color or animation.

Use a persistent semantic indicator such as:

- microphone/status icon;
- text label;
- elapsed time;
- waveform/activity indicator.

## 5. System appearance

Support:

- light mode;
- dark mode;
- increased contrast where applicable;
- reduced motion;
- text scaling;
- platform accent behavior where compatible with RimV identity.

Avoid hardcoded assumptions that reduce legibility under system appearance settings.

## 6. Typography

Prefer Apple system typography.

### macOS

Use system semantic text styles or their SwiftUI equivalents.

Avoid forcing iOS-sized controls and typography onto macOS.

### iOS

Use Dynamic Type-aware styles.

Never treat a fixed font size as the only valid layout.

Core content should adapt without clipping or losing essential controls.

## 7. Color

Use semantic RimV colors through shared tokens.

Do not use a literal color value repeatedly inside SwiftUI views.

Map semantic tokens to Apple-compatible colors.

Examples:

- `success-live` → active capture/live state;
- `ai` → AI-generated/AI-processing state;
- `warning` → warning;
- `error` → destructive/failure state.

When system accessibility settings change contrast requirements, legibility takes priority over exact brand color reproduction.

## 8. Materials and translucency

Use system materials carefully.

Appropriate uses may include:

- sidebars;
- popovers;
- sheets;
- overlays;
- transient chrome.

Do not stack multiple translucent surfaces unnecessarily.

Content must remain legible regardless of wallpaper/background.

## 9. Icons

Prefer SF Symbols on Apple platforms.

Use semantic symbol names rather than images copied from another platform.

Keep symbol weight and rendering mode consistent with nearby text and controls.

Typical RimV concepts:

- microphone;
- waveform;
- stop;
- play;
- transcript/captions;
- settings;
- download;
- model;
- warning;
- checkmark;
- sparkles/AI.

Do not use an icon alone when its meaning is not obvious.

## 10. Buttons

Use native button styles whenever possible.

### Primary action

There should usually be one visually dominant action in a local context.

Examples:

- Start Listening
- Start Transcription
- Download Model
- Retry

### Destructive action

Use the platform's destructive role rather than manually styling every destructive action.

Examples:

- Delete recording
- Remove model
- Clear history

Avoid oversized CTA-style web buttons in ordinary macOS utility interfaces.

## 11. Menus and commands

### macOS

Use the menu bar for conventional app commands:

- File;
- Edit;
- View;
- Window;
- Help;
- app-specific commands.

Provide keyboard shortcuts for frequent desktop actions.

Do not duplicate every menu command as a visible button.

### iOS

Use menus for secondary or compact command collections when appropriate.

Keep primary actions visible.

## 12. Sheets, popovers, and alerts

Use modal presentation only when the user must focus on a contained task or decision.

Avoid chains of modals.

Use:

- sheet for contained workflow;
- popover for lightweight contextual controls on suitable devices;
- alert for short important decisions/errors;
- confirmation dialog for destructive or mutually exclusive choices.

Do not display multiple alerts simultaneously.

## 13. Window design — macOS

RimV windows should:

- resize sensibly;
- define useful minimum sizes;
- preserve primary content visibility;
- avoid arbitrary fixed dimensions;
- remember state only when useful;
- keep toolbar/sidebar behavior stable.

If a window becomes narrow:

1. preserve primary content;
2. collapse secondary/contextual UI;
3. avoid shrinking text below readable size.

## 14. Menu bar / status item — macOS

For menu-bar functionality:

- status must be understandable at a glance;
- recording state must remain visible;
- menu items should be concise;
- destructive actions need separation from routine actions;
- quitting and settings should follow conventional placement;
- opening the full application should be obvious.

Do not overload the menu-bar popover with the full desktop application UI.

## 15. Navigation — iOS

Choose navigation based on information architecture.

Use:

- NavigationStack for hierarchical flows;
- tabs for stable top-level destinations;
- sheets for focused temporary tasks.

Do not create a hamburger menu merely to mimic a web interface.

## 16. Touch

iOS controls must have sufficiently large touch targets.

Do not shrink touch targets simply because visible icons are small.

Gestures must not be the only way to perform critical actions unless an accessible alternative is provided.

## 17. Keyboard — macOS

Desktop RimV workflows should support keyboard use.

Important actions may include:

- start/stop capture;
- search;
- open settings;
- navigation;
- close/dismiss;
- command palette or relevant app commands.

Avoid overriding familiar system shortcuts.

## 18. Focus

Focus order must follow the visual/interaction hierarchy.

Custom SwiftUI/AppKit views must not create focus traps.

When a modal closes, restore focus logically when practical.

## 19. Accessibility

Use native controls because they provide baseline accessibility semantics.

For custom UI:

- provide accessibility labels;
- provide values when state matters;
- provide hints only when useful;
- expose state changes appropriately;
- group elements when individual exposure creates noise.

Always test critical flows using VoiceOver.

Critical RimV accessibility states include:

- current microphone state;
- active recording state;
- model download progress;
- transcription status;
- error/recovery action;
- selected audio device.

## 20. VoiceOver labels

Labels should describe purpose, not appearance.

Good:
- `Start listening`
- `Stop recording`
- `Transcription progress, 72 percent`

Avoid:
- `Green button`
- `Microphone icon button`
- `Purple card`

## 21. Dynamic Type

On iOS:

- use semantic text styles;
- allow multiline labels where sensible;
- avoid clipping;
- verify large accessibility sizes;
- avoid fixed-height text containers.

If a layout breaks at large sizes, adapt the layout rather than disabling scaling.

## 22. Reduced motion

When Reduce Motion is enabled:

- remove decorative movement;
- minimize large spatial transitions;
- retain essential state feedback;
- replace motion with opacity/state changes where appropriate.

A live waveform may still communicate meaningful audio activity, but it should not become distracting.

## 23. Increased contrast and visible boundaries

Custom controls must remain distinguishable under accessibility contrast/border preferences.

When system preferences indicate stronger borders/contrast are needed:

- strengthen outlines;
- increase separation;
- do not rely on subtle surface differences alone.

## 24. Permissions

Microphone permission is a first-class RimV flow.

Before requesting permission:

- establish user intent;
- explain the feature context naturally;
- do not show a fake permission dialog.

When denied:

- explain that audio capture is unavailable;
- show a clear recovery path;
- link to system settings only when appropriate.

Do not repeatedly prompt after denial.

## 25. Privacy

If RimV processes audio locally, say so only when that behavior is actually guaranteed.

If data can leave the device, the interface must not imply local-only behavior.

Recording/transcription interfaces should make capture state obvious.

## 26. Errors

User-facing errors should answer:

1. What happened?
2. What can the user do?

Example:

`Microphone unavailable. Select another input device or check microphone access in System Settings.`

Avoid exposing raw Rust/OS errors as the primary UI message.

Diagnostic detail may be available separately.

## 27. Empty states

Empty states should guide the next action.

Examples:

- no transcripts → explain how to start;
- no model installed → offer download;
- no microphone detected → explain recovery;
- no history → avoid presenting an empty table with no context.

## 28. Loading/progress

For model downloads or long inference setup:

- show progress when measurable;
- use indeterminate progress only when progress cannot be measured;
- allow cancellation when cancellation is safe;
- explain what is being prepared.

## 29. SwiftUI state model

Keep view state explicit.

Prefer state categories such as:

```text
idle
permissionRequired
preparing
listening
transcribing
completed
failed
```

over scattered booleans like:

```text
isLoading
isRecording
hasError
isReady
```

when those booleans can produce impossible combinations.

## 30. SwiftUI view structure

A view should primarily describe UI.

Move:

- audio engine control;
- file IO;
- heavy transformation;
- model loading;
- Rust bridge details

into dedicated services/state layers.

Avoid performing expensive work in `body`.

## 31. Preview strategy

Provide previews for meaningful states when practical:

- idle;
- active capture;
- loading;
- error;
- long transcript;
- dark mode;
- large text.

Do not create previews that depend on a live microphone or production engine.

## 32. Localization readiness

Do not build layout around English-only string lengths.

Allow controls and labels to expand.

Avoid string concatenation that makes localization difficult.

## 33. Apple QA checklist

Before completing a meaningful Apple UI change:

- [ ] RimV design system followed
- [ ] platform conventions followed
- [ ] light mode checked
- [ ] dark mode checked
- [ ] primary state obvious
- [ ] permission-denied path exists
- [ ] loading/error/empty states handled
- [ ] keyboard flow checked on macOS
- [ ] touch targets checked on iOS
- [ ] VoiceOver labels checked
- [ ] Dynamic Type checked on iOS
- [ ] reduced motion considered
- [ ] no color-only communication
- [ ] modal usage justified
- [ ] platform-native icons used where appropriate
