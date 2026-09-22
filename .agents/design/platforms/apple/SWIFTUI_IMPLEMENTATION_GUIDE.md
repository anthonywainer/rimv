# SwiftUI Implementation Guide for RimV

## Purpose

Translate RimV's design system and Apple platform rules into maintainable SwiftUI.

## 1. Design tokens

Expose shared design semantics through a small token layer.

Example structure:

```text
Design/
├── RimVColor.swift
├── RimVSpacing.swift
├── RimVRadius.swift
├── RimVTypography.swift
└── RimVMotion.swift
```

Do not scatter literal colors and spacing values through feature views.

## 2. Semantic colors

Prefer:

```swift
RimVColor.live
RimVColor.ai
RimVColor.warning
RimVColor.error
```

instead of:

```swift
Color.green
Color.purple
Color.orange
Color.red
```

when the color has product semantics.

## 3. Feature composition

Suggested pattern:

```text
Feature/
├── FeatureView.swift
├── FeatureModel.swift
├── Components/
└── PreviewSupport/
```

Use the project's actual architecture if it already has a preferred structure.

Do not reorganize existing code solely to match this example.

## 4. State

Represent mutually exclusive lifecycle states as an enum where practical.

Example:

```swift
enum CaptureState {
    case idle
    case preparing
    case listening
    case transcribing
    case failed(String)
}
```

This prevents impossible boolean combinations.

## 5. Main actor

UI-observable state should update on the appropriate main actor/context.

Do not run model inference, audio processing, blocking file IO, or long Rust bridge calls on the UI thread.

## 6. Tasks

Keep long-running tasks cancellable.

Cancel work when:

- the user explicitly cancels;
- the owning screen/service ends and work is no longer needed;
- a replacement request supersedes previous work.

Avoid launching untracked tasks from views without understanding their lifetime.

## 7. Native controls

Prefer:

- `Button`
- `Toggle`
- `Picker`
- `Menu`
- `ProgressView`
- `NavigationStack`
- `List`
- `Form`

when they fit.

Custom implementations should have a concrete product need.

## 8. Accessibility modifiers

Use explicit modifiers where default semantics are insufficient.

Examples:

```swift
.accessibilityLabel(...)
.accessibilityValue(...)
.accessibilityHint(...)
```

Do not add redundant labels to controls already correctly described.

## 9. Reduced motion

Use environment accessibility settings where relevant.

When motion is decorative, disable or simplify it when reduced motion is enabled.

## 10. Text

Use semantic text styles and allow wrapping.

Avoid:

- fixed frame heights around dynamic text;
- tiny text for primary content;
- manual truncation of important error messages.

## 11. Lists and identity

Use stable IDs.

Do not use array index as identity when list items can be inserted, removed, or reordered.

## 12. Errors

Translate domain errors into user-facing UI states.

Keep raw diagnostic error available for logs or a detail view if useful.

## 13. Preview coverage

Useful preview states:

- normal;
- active capture;
- loading;
- error;
- dark;
- large text;
- long content.

Use mocks/stubs rather than live production resources.

## 14. AppKit/UIKit bridge

When using representables:

- expose accessibility;
- define coordinator ownership;
- clean up observers/delegates;
- avoid repeated object recreation;
- document lifecycle assumptions.

## 15. UI performance

Watch for:

- expensive formatting in `body`;
- unnecessary state invalidation;
- large synchronous transforms;
- image/audio work on main thread;
- unstable list IDs.

Measure before complex optimization.
