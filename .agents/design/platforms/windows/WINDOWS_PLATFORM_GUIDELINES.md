# RimV Windows Platform Guidelines

## Scope

Use these rules for RimV Windows desktop UI and native integration.

The shared design system remains the source of truth for product identity.

## 1. Native-first design

Prefer Windows-native patterns for:

- navigation;
- title bars;
- menus;
- dialogs;
- notifications;
- system tray;
- keyboard access;
- focus behavior;
- high-contrast support.

Use WinUI 3 / Windows App SDK conventions when that matches the project architecture.

Use lower-level Win32 APIs only where necessary for capabilities not exposed cleanly elsewhere.

## 2. Fluent adaptation

RimV may use Fluent principles such as:

- clear hierarchy;
- depth only where meaningful;
- subtle materials;
- consistent motion;
- visible focus;
- compact but comfortable desktop density.

Do not reproduce Fluent visuals mechanically if doing so conflicts with readability or RimV semantics.

## 3. Window structure

Use desktop-appropriate layouts.

Possible structure:

```text
Window
├── NavigationView / sidebar (only if justified)
├── Command/toolbar area
├── Main content
└── Optional contextual panel
```

Do not add a persistent sidebar when the product has only one main workflow.

## 4. Title bar

If using a custom or extended title bar:

- preserve drag regions;
- preserve system caption buttons;
- avoid placing essential controls under caption buttons;
- maintain correct behavior at different DPI/scaling levels;
- preserve high-contrast visibility.

Prefer standard title-bar behavior unless custom chrome has clear value.

## 5. Navigation

Use the simplest navigation model that fits the information architecture.

Possible patterns:

- NavigationView for persistent sections;
- tabs for closely related peer views;
- dialogs/sheets for focused secondary tasks;
- command bars for contextual actions.

Do not add multiple competing navigation models.

## 6. Command surfaces

Primary commands should be visible when frequently used.

Secondary commands may move into:

- command bar overflow;
- context menus;
- menus.

Avoid hiding critical start/stop recording controls behind overflow.

## 7. Buttons

Use native roles and states where possible.

Primary actions should be visually dominant only within their local context.

Destructive actions should use semantic red treatment and explicit labels.

Avoid giant marketing-style CTA buttons in normal desktop utility surfaces.

## 8. Controls

Prefer native:

- Button
- ToggleSwitch
- ComboBox
- TextBox
- Slider
- ProgressBar
- ProgressRing
- MenuFlyout
- ContentDialog

before building custom controls.

## 9. Focus

Windows users expect strong keyboard focus indication.

Requirements:

- visible focus rectangle;
- logical tab order;
- no keyboard traps;
- correct return of focus after modal dialogs;
- keyboard access to primary actions.

Do not remove native focus visuals without replacing them with equally clear feedback.

## 10. Keyboard

Desktop RimV should support common keyboard workflows.

Potential shortcuts:

- start/stop listening;
- open settings;
- focus search;
- copy transcript;
- switch views;
- dismiss dialogs.

Avoid overriding familiar Windows shortcuts.

## 11. Mouse and pointer

Hover may expose secondary affordances, but critical actions must remain discoverable without hover.

Pointer targets should remain comfortable at common scaling settings.

## 12. DPI and scaling

Windows UI must behave correctly at:

- 100%;
- 125%;
- 150%;
- 200% scaling.

Avoid pixel-perfect assumptions tied to one display.

Use layout systems that adapt to effective pixels rather than raw hardware pixels.

## 13. Typography

Prefer Segoe UI / system typography.

Use semantic sizing and maintain the RimV hierarchy.

Do not force Apple or web font stacks onto Windows.

## 14. Color and contrast

Use RimV semantic tokens.

Support:

- light mode;
- dark mode;
- high contrast;
- Windows theme changes.

Do not rely on subtle gray-on-gray distinctions that disappear in high contrast.

## 15. Mica, Acrylic, and materials

Use system materials only when they support hierarchy and the chosen Windows technology provides them reliably.

Good candidates:

- app background;
- transient surfaces;
- navigation areas.

Avoid layering multiple translucent materials that reduce readability.

## 16. Icons

Prefer Fluent/System iconography where practical.

Keep icon style consistent across the app.

Do not mix SF Symbols, arbitrary web icon packs, and Fluent icons in the same Windows surface.

## 17. System tray

If RimV uses a tray icon:

- show clear current status;
- expose start/stop actions where useful;
- provide open/show app;
- provide settings;
- provide exit;
- separate destructive or exit actions from routine actions.

Do not duplicate the full application in a tiny tray menu.

## 18. Notifications

Use native Windows notifications for meaningful events only.

Examples:

- model download finished;
- transcription export completed;
- serious recoverable error.

Do not use notifications for routine every-click feedback.

## 19. Dialogs

Use modal dialogs only for decisions that require user attention.

Avoid chains of dialogs.

Destructive confirmation should be explicit.

## 20. Recording state

The user must always be able to understand whether RimV is:

- idle;
- requesting permission;
- preparing;
- listening;
- recording;
- transcribing;
- processing;
- failed.

Do not signal live state through color alone.

Use text/icon/state indicators.

## 21. Permissions/privacy

For microphone access:

- request only when the user initiates a related feature;
- explain what is required;
- handle denied access;
- provide recovery guidance;
- avoid repeated prompting.

If privacy settings block access, guide the user to the correct Windows settings area only when needed.

## 22. Accessibility

Support:

- Narrator;
- keyboard-only use;
- visible focus;
- text scaling;
- high contrast;
- reduced motion where applicable;
- accessible names and roles.

Custom controls must expose proper automation properties.

## 23. Text scaling

Ensure content remains usable when text size is increased.

Avoid fixed-height text containers that clip.

## 24. Error messages

User-facing errors should explain:

1. what happened;
2. what the user can do.

Do not surface raw HRESULTs or Rust errors as the main message.

Keep technical details available separately when useful.

## 25. Empty states

Examples:

- no transcript yet → explain how to start;
- no model installed → offer model setup;
- no audio device → explain recovery;
- no history → show a helpful next action.

## 26. Progress

Use determinate progress when measurable.

Use indeterminate indicators only when total progress is unknown.

For long operations:

- show what is happening;
- allow cancellation where safe;
- avoid freezing the UI thread.

## 27. Native integration

Be explicit about:

- COM initialization;
- apartment model;
- window handles;
- callback lifetime;
- UTF-16 conversion;
- resource ownership;
- error translation.

## 28. Windows QA checklist

- [ ] RimV design tokens used
- [ ] light theme checked
- [ ] dark theme checked
- [ ] high contrast checked
- [ ] 125%+ scaling checked
- [ ] keyboard navigation works
- [ ] focus is visible
- [ ] Narrator labels checked
- [ ] recording state is unmistakable
- [ ] loading/error/empty states exist
- [ ] microphone denied flow exists
- [ ] system tray behavior is clear
- [ ] title bar remains usable
- [ ] no macOS-specific interaction leaks into Windows UI
