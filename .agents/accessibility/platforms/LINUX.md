# Linux — Accessibility Across Desktops

**Applies to:** RimV Linux UI when its toolkit is established. Do not assume GNOME/GTK, KDE/Qt, Wayland or X11 until the actual implementation is known.

## GTK / GNOME

- Prefer standard GTK controls, which expose baseline accessibility through `GtkAccessible` and AT-SPI.
- For custom widgets supply role, label, state and relationships explicitly.
- Test with Orca where supported and inspect accessible properties using toolkit tooling (for example GTK Inspector).
- Respect system font scaling and high-contrast theme; keep keyboard flow logical.

## Qt / KDE

- Prefer accessible built-in Qt controls.
- For custom controls expose accessible name, role, state and value using the toolkit's accessibility facilities (e.g., QAccessible where needed).
- Verify system font and palette changes, keyboard focus and screen-reader behavior.

## Desktop environment and sandboxing

- Wayland/X11 differences can affect global shortcuts and focus/system integration. Provide accessible in-app alternatives for core actions.
- Flatpak/portal restrictions can affect device and file access; treat permission-denied paths as first-class UI states.
- Do not rely on the system tray for the only way to stop recording; tray support varies.

## Test matrix

Record distribution, desktop environment, toolkit version, display server, audio server, packaging model and screen reader availability. At least one actual Linux accessibility test is needed before describing a Linux path as verified.

**Official references:**
- https://developer.gnome.org/documentation/guidelines/accessibility.html
- https://docs.gtk.org/gtk4/section-accessibility.html
- https://doc.qt.io/qt-6/accessible.html
