# Shell Layout, Responsive Hierarchy and Navigation

## Desktop layout: conditional, not mandatory

Hamu's documented desktop shell uses a left sidebar, main workspace and contextual right panel. RimV may use the same pattern *when* the feature set and available width support it.

| Region | Baseline |
|---|---|
| Sidebar | 240–280px (ideal 260) |
| Workspace | Flexible; aim for at least ~720px in three-column mode |
| Context/AI panel | 320–380px (ideal 350) |
| Column gap | 20–24px |
| Panel padding | 24px |

## Adaptive priority

1. Keep live recording state and Stop available.
2. Protect the transcript's legibility and text selection.
3. Show relevant AI results when space permits.
4. Collapse the optional right panel before compressing main content.
5. Collapse sidebar into a drawer, rail or navigation destination as appropriate.
6. Use one main column for mobile; secondary information becomes separate screens/sheets.

Do **not** shrink all UI text to maintain a three-column screenshot. Use content-driven breakpoints, not platform-specific fixed screen assumptions.

## Suggested responsive tiers for web preview only

- Wide: full three-region shell when minimum viable widths fit.
- Medium: main workspace + optional compact navigation; AI as drawer/panel.
- Narrow: single content column; navigation in header/drawer.

These are implementation patterns, not universal `px` breakpoints for native apps.

## Scroll behavior

- Transcript stream can scroll independently if the rest of the shell needs stability.
- Avoid nested scroll containers that trap wheel/touch/keyboard input.
- Prevent live-update scroll jumps when the user has scrolled to older text.
- Preserve focus and reading position on partial transcript updates.

## Navigation

- Current location must be perceivable without relying only on color.
- Preserve platform-native menu and keyboard conventions (expanded in Part 4B).
- Keep settings, model management and history visually secondary to the active recording workspace unless that is the task.
- Avoid hiding essential actions under hover-only menus.
