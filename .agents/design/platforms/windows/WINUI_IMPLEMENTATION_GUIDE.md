# WinUI / Windows App SDK Implementation Guide

## Purpose

Translate the RimV design system into maintainable Windows-native UI.

## 1. Design tokens

Create a central token layer for:

- semantic colors;
- spacing;
- radii;
- typography;
- motion.

Avoid scattering literal colors and spacing values through XAML/C++/C#/Rust bindings.

## 2. Resource dictionaries

Where appropriate, define shared resources through resource dictionaries.

Prefer semantic names such as:

```text
RimVBackgroundBrush
RimVSurfaceBrush
RimVLiveBrush
RimVAIBrush
RimVErrorBrush
RimVSpacingM
```

instead of purely visual names like:

```text
Green500
Purple600
Gray100
```

when the token carries product meaning.

## 3. Theme resources

Use theme-aware resources for light/dark/high contrast.

Do not implement theme switching by manually rewriting every control color.

## 4. View state

Represent mutually exclusive UI lifecycle states explicitly.

Example:

```text
Idle
Preparing
Listening
Transcribing
Completed
Failed
```

Avoid scattered booleans that can create impossible combinations.

## 5. Async work

Do not block the UI thread with:

- model loading;
- audio processing;
- filesystem work;
- long Rust FFI calls;
- network/download operations.

Keep cancellation available for long-running tasks when safe.

## 6. Native controls first

Use native controls before custom controls unless requirements justify otherwise.

## 7. Accessibility

For custom controls ensure:

- accessible name;
- role/control type;
- value/state;
- keyboard focus;
- high-contrast compatibility.

## 8. DPI

Never assume a fixed physical pixel size.

Use layout primitives and effective pixel units.

## 9. Rust bridge

When crossing Windows UI ↔ Rust:

- define thread ownership;
- avoid callbacks into destroyed UI;
- marshal UI updates to the UI thread;
- map errors into user-facing states;
- document ownership of buffers and strings.

## 10. Validation

Test:

- light/dark;
- high contrast;
- scaling;
- keyboard;
- Narrator where available;
- app suspend/resume/activation behavior if applicable;
- tray/notification behavior if implemented.
