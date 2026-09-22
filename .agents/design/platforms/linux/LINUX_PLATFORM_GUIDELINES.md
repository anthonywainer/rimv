# RimV Linux Platform Guidelines

## Scope

Use these rules for Linux desktop UI and native integration.

Linux is not a single visual environment.

Avoid assuming:

- GNOME only;
- KDE only;
- X11 only;
- Wayland only;
- one audio server;
- one package format;
- one theme.

## 1. Native-first adaptation

Prefer the project's chosen Linux toolkit and desktop conventions.

Possible toolkits may include:

- GTK/libadwaita;
- Qt/Kirigami;
- another established toolkit.

Do not introduce multiple GUI toolkits without a strong architectural reason.

## 2. Desktop environment awareness

RimV should preserve product semantics while respecting the current desktop environment.

Avoid hard-coding:

- icon shapes;
- title bar behavior;
- theme assumptions;
- font names;
- color palettes that fight system themes.

## 3. GNOME

If the Linux implementation uses GTK/libadwaita:

Favor:

- header bars;
- adaptive layouts;
- concise primary actions;
- native preferences patterns;
- system color/theme handling.

Avoid desktop UI that imitates Windows or macOS.

## 4. KDE

If using Qt/Kirigami:

Favor:

- KDE/Qt conventions;
- native menus/toolbars where appropriate;
- system theme integration;
- standard shortcuts.

Do not force GNOME-only patterns onto KDE users.

## 5. Window layout

Use the simplest structure appropriate to the app.

Possible pattern:

```text
Window
├── header/toolbar
├── optional navigation
├── main content
└── optional context panel
```

Keep primary transcript/capture content dominant.

## 6. Theme

Support system light/dark behavior where the selected toolkit allows it.

Avoid hard-coded surfaces that become unreadable in user themes.

## 7. Typography

Prefer system/toolkit fonts.

Do not bundle or force Apple/Windows fonts.

Maintain RimV hierarchy through semantic sizes and weights.

## 8. Icons

Prefer toolkit/system icon themes where appropriate.

Use symbolic icons for common actions.

Custom RimV icons should remain stylistically restrained.

## 9. Recording state

Always expose capture state with more than color.

Relevant states:

- idle;
- permission/access problem;
- preparing;
- listening;
- recording;
- transcribing;
- processing;
- error.

Use text/icon/status indicator.

## 10. Audio stack

Do not assume one audio stack.

Possible environments include:

- PipeWire;
- PulseAudio compatibility;
- ALSA.

Prefer the modern user-configured stack where practical.

Do not bypass the desktop audio stack unless technically required.

## 11. Device selection

Device selection UI should use human-readable device names.

Handle:

- default device changes;
- device disconnect;
- device reconnect;
- unavailable input;
- permission/access errors.

## 12. Wayland/X11

Do not assume behavior available on X11 also exists on Wayland.

Features that may differ include:

- global shortcuts;
- window positioning;
- screen capture;
- tray integration;
- input hooks.

Detect capabilities rather than assuming.

## 13. Global shortcuts

Global hotkeys may depend on:

- desktop environment;
- compositor;
- portal support.

Provide graceful degradation if unsupported.

## 14. System tray

Tray support varies across desktop environments.

Do not make core functionality depend solely on tray availability.

If tray is supported:

- show concise status;
- expose routine capture actions;
- provide open/settings/quit.

## 15. Notifications

Use desktop notification mechanisms for meaningful events.

Do not flood the desktop with routine notifications.

## 16. File dialogs

Use toolkit/native or portal-backed file dialogs when possible.

Prefer portals in sandboxed contexts.

## 17. Portals

Where relevant, use XDG Desktop Portal abstractions for:

- file access;
- notifications;
- screen capture;
- global shortcuts;
- sandbox-friendly integration.

## 18. Accessibility

Support toolkit accessibility mechanisms.

Check:

- keyboard navigation;
- focus visibility;
- screen reader semantics;
- text scaling;
- high-contrast themes;
- reduced motion when exposed by environment/toolkit.

## 19. Keyboard

Support common desktop keyboard workflows.

Avoid stealing established shortcuts.

## 20. Pointer

Critical actions must not depend on hover.

Context menus should remain supplemental.

## 21. Errors

User-facing errors should explain:

1. what happened;
2. what to do next.

Avoid exposing only ALSA/PipeWire/DBus/Rust raw errors.

## 22. Packaging awareness

UI behavior should not assume one packaging environment.

Possible formats:

- Flatpak;
- AppImage;
- distro package;
- tar archive.

Sandboxed packaging may affect:

- filesystem access;
- portals;
- device access;
- DBus;
- notifications.

## 23. Sandboxing

If distributed through Flatpak or another sandboxed format:

- request minimal permissions;
- prefer portals;
- document capability limitations;
- test actual sandbox behavior.

## 24. Scaling

Test with:

- standard scaling;
- HiDPI;
- fractional scaling where supported.

Avoid raw pixel assumptions.

## 25. Localization

Do not size controls around English-only labels.

Allow expansion.

## 26. Linux QA checklist

- [ ] RimV design semantics preserved
- [ ] system theme respected
- [ ] system fonts respected
- [ ] keyboard navigation works
- [ ] focus visible
- [ ] screen reader semantics considered
- [ ] Wayland behavior checked where relevant
- [ ] X11 behavior checked where relevant
- [ ] PipeWire/PulseAudio assumptions verified
- [ ] tray is not required for core functionality
- [ ] portal behavior checked if sandboxed
- [ ] HiDPI/fractional scaling considered
- [ ] no GNOME-only assumptions unless app explicitly targets GNOME
- [ ] no KDE-only assumptions unless app explicitly targets KDE
