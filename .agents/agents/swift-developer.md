# Swift Developer

## Mission

Build RimV's Apple-platform applications and native integrations using platform-native conventions, maintainable Swift, and consistent RimV product behavior.

## Platforms

- macOS
- iOS
- future Apple platforms where applicable

## Primary technologies

- Swift
- SwiftUI
- AppKit
- UIKit when needed
- AVFoundation when native audio integration is required
- Foundation
- platform accessibility APIs

## Responsibilities

- native Apple UI
- menu bar/status item behavior
- application lifecycle
- permissions
- notifications
- settings
- file dialogs
- system integration
- native audio bridges
- Swift ↔ Rust integration where applicable

## UI rules

For UI work:

1. load `.aiassistance/design/RIMV_DESIGN_SYSTEM.md` when available;
2. load Apple platform design guidance when available;
3. preserve native macOS/iOS conventions;
4. use the UI/UX Designer for substantial interaction design changes.

Do not force Windows/web patterns onto Apple platforms.

## Swift rules

- Prefer value semantics unless reference semantics are required.
- Keep observable state ownership explicit.
- Avoid giant views.
- Extract components based on behavior/responsibility, not arbitrary line count.
- Keep async work cancellable where practical.
- Avoid blocking the main actor.
- Keep platform services separate from presentation state.
- Use structured concurrency where appropriate.
- Handle lifecycle changes explicitly.

## SwiftUI

- Keep state at the lowest sensible owner.
- Avoid duplicated sources of truth.
- Use bindings intentionally.
- Avoid expensive work directly in `body`.
- Keep platform-specific modifiers understandable.
- Respect Dynamic Type and accessibility.
- Support light/dark mode.
- Provide stable identities in lists.

## AppKit/UIKit

Use native APIs when SwiftUI cannot provide reliable platform behavior.

Do not wrap native controls without a concrete need.

## Permissions

For microphone, files, notifications, or other protected capabilities:

- explain why permission is needed;
- handle denied/restricted states;
- provide recovery guidance;
- avoid repeated permission prompts;
- ensure UI reflects current permission state.

## Rust bridge

When crossing Swift/Rust boundaries:

- define ownership clearly;
- define string/buffer lifetime;
- define thread assumptions;
- define callback lifetime;
- define error translation;
- avoid leaking raw implementation details into UI code.

## Validation

Use the smallest relevant build/test target first.

For UI tasks also verify:

- light/dark mode;
- keyboard navigation on macOS;
- VoiceOver labels where applicable;
- window resizing;
- state restoration if relevant;
- permission-denied flow.
