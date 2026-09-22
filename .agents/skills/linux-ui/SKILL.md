# Skill: Linux UI Development

## Use when

The task affects RimV Linux UI, desktop integration, tray/notification behavior, Linux accessibility, or Linux-specific interaction patterns.

## Required references

Read only what applies:

1. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
2. `.aiassistance/design/platforms/linux/LINUX_PLATFORM_GUIDELINES.md`
3. `GNOME_GUIDELINES.md` if using GTK/libadwaita
4. `KDE_GUIDELINES.md` if using Qt/KDE/Kirigami
5. `LINUX_UI_REVIEW_CHECKLIST.md` for review

## Procedure

### 1. Identify target environment

Determine:

- toolkit;
- desktop environment assumptions;
- Wayland/X11 implications;
- packaging model;
- audio stack assumptions.

Do not guess if these materially affect implementation.

### 2. Inspect existing project patterns

Reuse current toolkit, theme, components, and architecture.

Do not introduce a second GUI toolkit casually.

### 3. Apply RimV semantics

Preserve shared:

- live;
- AI;
- warning;
- error;
- typography hierarchy;
- spacing rhythm.

### 4. Respect Linux environment

Check:

- system theme;
- fonts;
- scaling;
- keyboard;
- portals;
- tray availability;
- desktop-specific behavior.

### 5. Accessibility

Verify:

- focus;
- keyboard navigation;
- accessible labels/roles;
- large font behavior;
- contrast.

### 6. Audio/system integration

When relevant, verify:

- PipeWire/PulseAudio/ALSA assumptions;
- device changes;
- permission/access errors;
- sandbox limitations.

### 7. Validate

Document the tested environment where behavior is environment-sensitive.

## Completion

Report:

- what changed;
- environment validated;
- unresolved compatibility limitation if any.
