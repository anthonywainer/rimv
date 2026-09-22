# Windows — Accessibility Implementation and Testing

**Applies to:** native Windows UI, WinUI/Windows App SDK, Win32 integrations, and WebView2 when used.

## UI Automation and Narrator

- Prefer native controls that already expose UI Automation semantics.
- For custom controls verify Name, ControlType/role, IsEnabled, focus, value/range/state and supported control patterns as applicable.
- Keep visible captions aligned with accessible names (important for speech control).
- Use meaningful status/live region information for capture and model progress. Test with Narrator to prevent repeated chatter.
- Use `AutomationProperties` or the framework's equivalent when the default metadata is incomplete; avoid redundant overrides.

## Input and visibility

- Test Windows Tab/Shift+Tab and native arrow-key patterns in composite controls.
- Check 100%, 125%, 150%, and higher DPI/text scaling when available; keep Stop and Retry accessible.
- Ensure high-contrast/forced-color behavior doesn't hide focus, selection or status boundaries.
- Use Fluent/native focus visuals unless a carefully tested replacement is necessary.

## System surfaces

- If RimV exposes tray actions, give concise names and ensure the full application remains usable without tray-only interaction.
- Notifications must not be the only source of an important error/recovery action.
- Title-bar extensions must leave system controls, dragging and keyboard access intact.

## Audio device and permission errors

- Display a persistent human-readable cause/remedy.
- Map error state to accessible descriptive text, not just a yellow/red icon or raw HRESULT.
- Do not claim capture stopped until the engine has confirmed it.

## Validation

Manual: Narrator, keyboard only, high contrast, increased text, DPI scaling, device disconnect and denied access. Tools: Accessibility Insights for Windows, Inspect (UIA tree) and framework-specific UI testing where available. Automated tools supplement manual tests.

**Official references:**
- https://learn.microsoft.com/en-us/windows/apps/develop/accessibility
- https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-testing
