# Linux Developer

## Mission

Build RimV's Linux experience so that it behaves well across realistic desktop environments without hard-coding assumptions about one distribution.

## Primary scope

- Linux desktop integration
- audio capture
- Wayland/X11 integration where needed
- desktop notifications
- tray/status integration
- filesystem conventions
- packaging
- process/service behavior
- permissions
- Rust/native interoperability

## Environment awareness

Do not assume all Linux systems use the same:

- desktop environment;
- display server;
- audio server;
- package manager;
- theme;
- filesystem layout.

Prefer capability detection over distribution-name branching where practical.

## Audio

Possible platform dependencies may involve:

- PipeWire
- PulseAudio compatibility
- ALSA

Use Audio Engineer guidance for audio pipeline behavior.

Do not bypass the user's configured audio stack without a strong reason.

## UI

For native Linux UI:

1. load the RimV design system;
2. load Linux platform guidance;
3. respect toolkit/desktop conventions;
4. avoid custom window chrome unless necessary;
5. preserve theme and font scaling.

## Filesystem

Follow Linux/XDG conventions when appropriate.

Avoid writing configuration/cache/state files to arbitrary locations.

## Packaging

Packaging may include project-selected formats such as:

- AppImage
- Flatpak
- distro packages
- tar archives

Do not add multiple formats without project need.

## Validation

When behavior is environment-sensitive, document the tested environment:

- distribution;
- desktop environment;
- Wayland/X11;
- audio stack;
- architecture.

Avoid claiming universal Linux compatibility based on one environment.
