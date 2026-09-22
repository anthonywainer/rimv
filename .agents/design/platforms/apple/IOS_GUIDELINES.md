# RimV iOS Design Guidelines

Use together with:

- `RIMV_DESIGN_SYSTEM.md`
- `APPLE_PLATFORM_GUIDELINES.md`

## Product character

RimV on iOS should be touch-first, focused, and state-driven.

Do not compress the desktop application into a phone layout.

## Primary flow

The main capture/transcription experience should make the current state obvious.

A typical information priority is:

1. capture/listening status;
2. main transcript/content;
3. primary action;
4. AI result/context;
5. secondary controls.

## Navigation

Use `NavigationStack` for hierarchy.

Use tabs only when RimV has multiple persistent top-level destinations that users switch between frequently.

Do not use tabs for temporary stages of one workflow.

## Sheets

Use sheets for:

- model selection;
- input selection;
- focused settings;
- export/share options;
- short setup flows.

Avoid excessive sheet nesting.

## Touch targets

Interactive targets should be comfortably tappable.

Small visual icons may use larger invisible hit areas.

## Dynamic Type

Every important screen must tolerate larger text sizes.

At large accessibility sizes:

- allow controls to stack vertically;
- let labels wrap;
- prioritize content;
- avoid fixed-height rows.

## Safe areas

Do not place controls under:

- home indicator;
- sensor housing;
- system bars

unless using intentional system-supported layouts.

## Keyboard

When text input appears:

- choose appropriate keyboard type;
- provide sensible submit/done behavior;
- avoid covering critical controls;
- restore context when the keyboard dismisses.

## Haptics

Use haptics only where they provide meaningful confirmation.

Potential examples:

- successful start/stop;
- important selection;
- destructive confirmation.

Do not add haptics to every tap.

## Recording state

The active recording/listening state must remain visible even if the user navigates within a relevant flow.

If capture continues in the background or after navigating away, communicate that clearly.

## Interruptions

Handle interruption cases such as:

- phone/audio session interruption;
- device route change;
- app backgrounding;
- permission changes.

The UI must reflect actual engine state after interruption.

## Share/export

Use the system share experience rather than building a custom destination picker unless product requirements demand one.

## Accessibility

Test:

- VoiceOver;
- large Dynamic Type;
- button shapes/contrast preferences where relevant;
- reduced motion;
- landscape if supported.

Avoid gesture-only critical functionality.
