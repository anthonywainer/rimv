# KDE / Qt / Kirigami Guidance

Use only if RimV's Linux implementation targets Qt/KDE/Kirigami.

## Design character

Favor:

- standard Qt/KDE controls;
- system theme integration;
- familiar menus/toolbars;
- standard keyboard behavior;
- KDE icon/theme integration.

## Navigation

Use Kirigami/Qt navigation patterns appropriate to the selected application structure.

Do not imitate GNOME header-bar behavior if it conflicts with the chosen Qt shell.

## Settings

Use clear categorized settings where necessary.

Avoid deep nested settings trees for a small number of options.

## Theme

Respect user theme, palette, font, and scaling.

Avoid hard-coded colors that conflict with KDE themes.

## Accessibility

Expose accessible names/roles through the toolkit.

Verify keyboard navigation and focus order.
