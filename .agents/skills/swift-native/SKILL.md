---
name: swift-native
description: "Use for Swift, SwiftUI, AppKit, UIKit, native Apple integration, permission flows and Swift-to-Rust bridges for macOS or iOS. Not for web UI or Windows/Linux native tasks."
---

# Swift and Apple Native Development

## Goal
Implement Apple-native RimV experiences without leaking platform implementation into the shared Rust core.

## Choose the appropriate API
- SwiftUI for modern declarative screens and reusable views.
- AppKit for macOS menu-bar, window, menu or other functionality SwiftUI does not model reliably.
- UIKit when iOS platform behavior requires it.
- AVFoundation and system privacy APIs only when native capture/session integration is needed.

## Workflow
1. Establish target OS and minimum supported version; check actual app targets rather than assume latest APIs.
2. Identify state owner and platform service owner. UI state is not the same as microphone session/model state.
3. For UI work, consult `.aiassistance/design/RIMV_DESIGN_SYSTEM.md` **when present** and Apple guidance from Part 4.
4. Use native menus, toolbar locations, navigation and keyboard shortcuts appropriate to each platform.
5. Keep blocking decoding, inference and filesystem work off the main actor.
6. For Swift/Rust bridges, specify pointer/buffer lifetime, string encoding, error mapping and callback thread guarantees.
7. Verify denial, interruption, inactive app, sleep/wake and shutdown paths when relevant.

## State and concurrency
Prefer one authoritative observable state model per feature. Do not mutate UI-observed state from an arbitrary native callback thread. Make long-lived tasks cancellable and avoid retaining views/controllers through unmanaged callback cycles.

## Quality checks
- macOS: resizable windows, focus rings, menu shortcuts, VoiceOver, permission recovery.
- iOS: safe areas, touch targets, Dynamic Type, accessibility, audio session interruption/background restrictions.
- Both: light/dark appearance, reduced motion, language expansion and model download progress.

## Avoid
Using a web UI pattern merely because it looks similar, suppressing a permission error, retaining raw Rust pointers beyond their lifetime, and performing inference on the UI thread.

## Exit
The code builds against the actual target; permission/error/cancellation paths are covered; platform behavior follows system conventions.
