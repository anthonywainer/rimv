# Workflow: UI Implementation

Use this workflow for implementing or modifying RimV user interfaces.

## 1. Identify scope

Determine:

- target platform;
- affected screen or flow;
- primary user goal;
- current implementation;
- relevant existing components.

## 2. Load only relevant guidance

Use:

1. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
2. the target platform guide
3. the relevant UI implementation skill
4. accessibility/usability guidance only when needed

Do not load all platform guides.

## 3. Inspect existing patterns

Before creating new UI:

- search for equivalent components;
- reuse design tokens;
- preserve local architecture;
- avoid introducing a new UI framework without explicit need.

## 4. Define states

Cover relevant states such as:

- idle;
- loading/preparing;
- active;
- success;
- empty;
- permission denied;
- error/retry.

## 5. Implement

- use semantic tokens;
- preserve native platform behavior;
- keep primary action clear;
- maintain keyboard/touch accessibility;
- avoid color-only status communication.

## 6. Validate

Check the smallest relevant target first.

For UI changes also verify:

- light/dark appearance;
- responsive/adaptive behavior;
- focus/keyboard or touch;
- text scaling;
- accessibility semantics;
- error and permission states.

## 7. Complete

Report the implementation result, validation performed, and any unresolved UI/accessibility issue.
