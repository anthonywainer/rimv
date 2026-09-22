# Design Token Adapters — One Source, Native Mappings

**Source:** `../tokens/rimv.tokens.json` from Part 4A. Never maintain another independently edited color table for each platform. The token values are a **proposed RimV baseline derived from Hamu**, not proof of existing RimV implementation.

## 1. Canonical schema

Read directly from the actual Part 4A token file:

- `color.light.<semantic-name>.$value` and `color.dark.<semantic-name>.$value`
- `semanticMessages.light|dark.<state>.surface|text.$value`
- `space.<scale>.$value`, `radius.<name>.$value`
- `size.<name>.$value`, `motion.<name>.$value`
- `typography.<style>` (`fontSize`, `lineHeight`, `fontWeight`)

Relevant colors include `background`, `surface`, `surface-secondary`, `border`, `text-primary`, `text-secondary`, `brand`, `action-solid`, `on-action`, `live`, `ai`, `warning`, `error` and `focus`. Use `action-solid` (not the lighter `brand`) for small white-label filled buttons in the light theme; the design system explains its contrast rationale.

## 2. Adapter contract

Each platform exposes **semantic names** to components and maps those names to suitable platform-native values. The meaning remains fixed, but accessibility/system appearance may modify the realized color/measurement. Follow a consistent naming convention such as `rimv.color.live` rather than a literal `green500` in feature code.

| Token group | macOS / iOS | Windows | Linux | Web |
|---|---|---|---|---|
| Colors | SwiftUI `Color` in a centralized theme; appearance-aware assets | Theme resources / XAML brush resources if WinUI | Toolkit palette/semantic color provider (GTK or Qt) | CSS custom properties; Tailwind maps to variables |
| Typography | Semantic system font styles; Dynamic Type on iOS | Segoe/system; text-scale-aware resources | System/toolkit fonts and user scaling | `rem`/relative text scale, sensible system stack |
| Spacing/radius | Layout constants with native overrides where justified | Resource values/effective pixels | Toolkit dimensions; user scaling | CSS variables/Tailwind utilities |
| Motion | Respect Reduce Motion | Respect Windows animation/accessibility settings | Respect toolkit/desktop setting where exposed | `prefers-reduced-motion` |
| Focus | Native focus styling or enhanced semantic focus | Native keyboard focus/high contrast | Toolkit focus and theme support | `:focus-visible` clearly exposed |

## 3. Conversion rules

1. Do **not** assume `px` in the JSON equals hardware pixels or should be used literally on every native platform; treat these as logical design reference units.
2. Keep platform defaults for native control heights, typography, safe areas and interactions when appropriate.
3. Respect user-selected larger type, high-contrast, increased contrast, reduced transparency and reduced motion.
4. If a source token does not meet contrast in a particular context, pick an accessible mapping and record the deviation if it changes the product's visible identity.
5. Separate semantic `live` from generic `success`: recording-active should never imply successful completion.
6. Make color-only changes testable with a visible text/icon state.

## 4. Web adapter example (illustrative; use actual token data)

```css
:root {
  --rimv-bg: #EEF2FA;
  --rimv-surface: #FFFFFF;
  --rimv-text-primary: #171827;
  --rimv-live: #16A34A;
  --rimv-ai: #7C3AED;
  --rimv-action: #6D4DE6;
  --rimv-on-action: #FFFFFF;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --rimv-bg: #0B0F19;
    --rimv-surface: #111827;
    --rimv-text-primary: #F9FAFB;
    --rimv-live: #22C55E;
    --rimv-ai: #A78BFA;
    --rimv-action: #2F3747;
    --rimv-on-action: #F9FAFB;
  }
}
```

Do not copy this snippet blindly: it illustrates mapping from the actual 4A token values and omits many tokens. The project may already implement equivalent CSS variables. Respect existing code before migrating it.

## 5. Native adapter pseudo-interfaces

### Apple

```text
RimVTheme.color(.live, appearance: .lightOrDark)
RimVTheme.color(.ai, appearance: .lightOrDark)
RimVTheme.spacing(.medium)
```

SwiftUI can use semantic `Color` properties plus `@Environment(\.colorScheme)` where needed. Prefer dynamic assets/native color behavior when appropriate. Do not bundle Apple fonts for other platforms.

### Windows

```text
RimVLiveBrush       → theme-specific brush
RimVAIContentBrush  → theme-specific brush
RimVActionBrush     → theme-specific brush
```

Use suitable theme-resource mechanisms; in Windows high contrast let system/theme semantics override insufficient custom colors.

### Linux

```text
RimVTheme.live      → toolkit-appropriate semantic color
RimVTheme.ai        → toolkit-appropriate semantic color
```

GTK/libadwaita and Qt/Kirigami require different implementations; do not force one Linux adapter on both. Respect system theme overrides.

## 6. Safe rollout

- Inventory existing tokens/styles per platform; do not assume they match Part 4A yet.
- Agree on canonical token names and references.
- Convert one representative component (for example the recording status) per supported UI.
- Test light/dark/high contrast, then expand progressively.
- For each changed token, verify all affected platforms; if unavailable locally, mark **Not verified** and request platform CI/manual review.
- If Part 4A's `RIMV_DESIGN_SYSTEM.md` and token JSON disagree, resolve in those canonical files before adapting downstream code.
