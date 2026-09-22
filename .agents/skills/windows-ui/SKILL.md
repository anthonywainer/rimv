# Skill: Windows UI Development

## Use when

The task affects RimV Windows UI, native desktop behavior, tray integration, Windows accessibility, or Windows-specific interaction patterns.

## Required references

Read only what is needed:

1. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
2. `.aiassistance/design/platforms/windows/WINDOWS_PLATFORM_GUIDELINES.md`
3. `WINUI_IMPLEMENTATION_GUIDE.md` for WinUI/Windows App SDK implementation
4. `WINDOWS_UI_REVIEW_CHECKLIST.md` for UI review

## Procedure

### 1. Identify the user flow

Clarify:

- primary user goal;
- current app state;
- primary action;
- failure/recovery path.

### 2. Inspect existing Windows patterns

Before creating new UI:

- find existing shared controls/tokens;
- preserve current project architecture;
- avoid introducing a second Windows UI framework.

### 3. Apply RimV semantics

Use design tokens for:

- live;
- AI;
- warning;
- error;
- surfaces;
- typography.

### 4. Apply Windows-native behavior

Check:

- title bar;
- command surfaces;
- keyboard;
- focus;
- DPI;
- high contrast;
- system tray/notification behavior if relevant.

### 5. Accessibility

Verify:

- Narrator name/value;
- tab order;
- visible focus;
- high contrast;
- text scaling.

### 6. Validate state coverage

Relevant states may include:

- idle;
- permission/access issue;
- preparing;
- live;
- transcribing;
- complete;
- error;
- empty.

### 7. Validate at multiple scaling levels

At minimum consider:

- 100%;
- 125%;
- 150%.

## Completion

Report only:

- key implementation result;
- validation performed;
- unresolved Windows-specific issue.
