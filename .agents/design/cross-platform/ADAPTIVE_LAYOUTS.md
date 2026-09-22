# Adaptive Layout and Interaction Parity

## Rule: preserve task, not geometry

The main capture control, true recording status and transcript are the core task. Secondary panels, model settings, advanced diagnostics and AI summaries may change placement across platforms and viewports. Do not recreate a desktop screenshot at phone size.

## Layout intentions

| Environment | Primary priority | Secondary placement | Interaction model |
|---|---|---|---|
| Large macOS / Windows / Linux desktop | Capture + transcript | Side inspector when space justifies it | Keyboard/pointer; native menus |
| Narrow desktop / tablet window | Capture + transcript | Collapsible side panel or separate section | Adapt menus/panels before shrinking content |
| iOS phone | Active status + transcript + stop | Navigation destination or sheet | Touch, Dynamic Type, safe areas |
| Large browser window | Capture + transcript | Responsive nav/aside if relevant | Keyboard/pointer; semantic HTML |
| Narrow browser / high zoom | Capture + transcript single column | Drawer, disclosure or separate view | Reflow, no critical off-screen actions |
| Menu-bar / tray popover | Current status + start/stop + open app | Full transcript in main app | Minimal contextual commands |

These are design patterns if the corresponding UI exists, not a mandate to implement all platforms/layouts now.

## Adaptive behavior by component

**Transcript:** use readable line length; long content scrolls independently only where focus/scroll semantics remain intuitive. Do not auto-scroll if the user intentionally scrolled back. If active capture continues, status/stop must remain discoverable.

**AI insights:** inline below transcript or in an inspector on wide screens; sheet/tab/expandable panel on compact screens. Keep AI text explicitly labeled; don't replace source transcript without user choice.

**Sidebar:** persistent only where navigation complexity warrants it. On small windows, collapse before sacrificing transcript font size.

**Status/control:** minimize dense toolbars at phone widths; one clear primary capture action. Desktop may additionally expose shortcuts and toolbar commands.

**Settings:** platform-native forms and menus; values persist consistently when shared by the product, with accurate availability notes where a preference is platform-specific.

## Accessibility-driven reflow

- Increased font sizes, large browser zoom, OS text scaling and long localized strings must reflow content.
- Keep focus order aligned with visual order after responsive changes.
- Prefer logical CSS/constraint layout to absolute positioning.
- Touch targets follow the platform; small visuals can have larger hit areas.
- Support reduced motion without removing essential feedback.
- Preserve high-contrast semantics, not exact RGB fidelity.

## Layout review scenarios

1. Minimum useful window width (per target platform).
2. Typical desktop width.
3. Phone portrait and landscape if the product supports both.
4. Browser at 200% zoom and narrow viewport, when Web is targeted.
5. Dynamic Type/accessibility sizes on iOS.
6. 125%/150% Windows scaling and large fonts on Linux.
7. Long transcript, long localized action label, error plus retry button.
8. Keyboard-only with a visible focused control; dialog close and focus return.

Don't assign a global minimum window width to all platforms; define it within the implementation and test it.
