# GNOME / GTK / libadwaita Guidance

Use only if RimV's Linux implementation targets GTK/libadwaita.

## Design character

Favor:

- clean header bars;
- adaptive layouts;
- restrained action density;
- simple preferences;
- clear navigation;
- native theme behavior.

## Header bars

Place frequent window-level actions in the header bar when appropriate.

Do not overcrowd it.

## Preferences

Use grouped preferences patterns rather than inventing a custom dashboard for simple settings.

## Adaptive layout

If the app window becomes narrow:

- collapse secondary regions;
- preserve transcript readability;
- avoid tiny controls.

## System theme

Allow libadwaita/GTK theme mechanisms to handle system appearance.

Do not manually force exact surface colors if doing so breaks system integration.

## Accessibility

Use GTK semantics and proper labels.

Keyboard and screen-reader access must work without custom hacks.
