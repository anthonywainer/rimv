# Apple — macOS and iOS Accessibility

**Applies to:** SwiftUI, AppKit or UIKit RimV interfaces. Use Part 4B.1 Apple design rules for native layout and interaction; use the canonical shared token system for visual semantics.

## macOS

- Prefer native controls with appropriate accessibility names, roles and states.
- Verify keyboard navigation, full keyboard access where supported, logical focus and VoiceOver reading order.
- Keep active capture state visible in the main window and menu-bar/status UI when those surfaces can control recording.
- Menu-bar-only actions must not become an inaccessible single path for core operation.
- Test resizing, Reduce Motion and Increase Contrast. Don't clip large system text.
- For custom AppKit controls, provide accurate accessibility information and relationships rather than assuming a painted view is automatically usable.

## iOS

- Prefer SwiftUI native controls and semantic font styles; support Dynamic Type up to larger accessibility sizes.
- Make visible targets comfortably touchable; RimV's 44pt-or-larger touch recommendation is a product target, not a statement of WCAG web requirements.
- Verify VoiceOver order, labels, traits, values and appropriate announcements.
- Test Voice Control where practical; button accessible names should contain visible names.
- Respect safe areas, orientation where relevant, Reduce Motion and Increased Contrast.
- Avoid gesture-only destructive actions; provide a visible accessible alternative.

## Recording and interruptions

- Distinguish preparing, active, paused, interrupted, stopping and confirmed stopped.
- When an audio session is interrupted or device route changes, announce the resulting *real* capture state once.
- Keep Stop discoverable while capture is active. Do not announce every live transcript token by default.
- Explain denied microphone access and expose recovery guidance; do not repeatedly invoke system permission prompts.

## Focus and presentation

- Sheets and alerts should use native focus conventions; after dismissal, return logically to the original workflow.
- Avoid auto-changing focus when new transcript text arrives or a progress bar increments.
- Group accessibility elements only when grouping improves comprehension without hiding essential actions.

## Validation

Use actual device/simulator capabilities as available: VoiceOver, Dynamic Type, light/dark, Increase Contrast, Reduce Motion, keyboard on macOS, and permission-denied and interruption flows. Record the hardware/OS and anything untested.

**Official reference:** https://developer.apple.com/accessibility/
