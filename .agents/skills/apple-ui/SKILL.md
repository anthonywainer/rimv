# Skill: Apple UI Development

## Use when

A task changes RimV UI or UX on:

- macOS;
- iOS;
- SwiftUI;
- AppKit;
- UIKit.

## Do not use when

The task is purely:

- Rust backend;
- audio DSP;
- Windows;
- Linux;
- web UI.

## Required references

Read only what is relevant:

1. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
2. `.aiassistance/design/platforms/apple/APPLE_PLATFORM_GUIDELINES.md`
3. `MACOS_GUIDELINES.md` or `IOS_GUIDELINES.md`
4. `SWIFTUI_IMPLEMENTATION_GUIDE.md` when implementing SwiftUI

## Procedure

### 1. Identify platform

Determine whether the task targets:

- macOS;
- iOS;
- both.

Do not assume the two platforms need identical UI.

### 2. Identify user goal

State internally:

- primary user goal;
- primary action;
- important system state;
- failure/recovery state.

### 3. Check existing UI

Before designing a new component:

- search for an existing equivalent;
- reuse existing tokens/components;
- preserve local architectural patterns.

### 4. Apply RimV semantics

Ensure:

- live state uses live semantics;
- AI content uses AI semantics;
- destructive/error states are distinct;
- visual identity remains consistent.

### 5. Apply native platform pattern

Choose native:

- navigation;
- window/sheet/popover;
- button role;
- menu;
- keyboard/touch interaction.

### 6. Accessibility

Check:

- label/value;
- focus;
- keyboard or touch;
- VoiceOver;
- Dynamic Type on iOS;
- reduced motion;
- contrast.

### 7. State coverage

Implement relevant:

- idle;
- loading/preparing;
- active;
- success;
- empty;
- permission denied;
- error/retry.

### 8. Validate

For macOS check:

- resizing;
- keyboard;
- light/dark;
- VoiceOver.

For iOS check:

- touch size;
- Dynamic Type;
- safe areas;
- VoiceOver;
- interruption/background behavior when relevant.

## Completion

Report only:

- relevant UI change;
- validation performed;
- unresolved platform/accessibility issue if any.
