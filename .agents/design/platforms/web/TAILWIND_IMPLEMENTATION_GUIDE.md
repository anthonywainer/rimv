# Tailwind CSS Implementation Guide for RimV

## Purpose

Apply RimV's design system through Tailwind without turning the codebase into duplicated utility strings.

## 1. Semantic variables first

Define design tokens as CSS custom properties.

Example:

```css
:root {
  --rimv-bg: #f8fafc;
  --rimv-surface: #ffffff;
  --rimv-text-primary: #111827;
  --rimv-live: #10b981;
  --rimv-ai: #7c3aed;
}

.dark {
  --rimv-bg: #0b0f19;
  --rimv-surface: #111827;
  --rimv-text-primary: #f9fafb;
  --rimv-live: #22c55e;
  --rimv-ai: #8b5cf6;
}
```

Use the canonical values from `RIMV_DESIGN_SYSTEM.md`.

## 2. Tailwind mapping

Where useful, map semantic CSS variables into Tailwind configuration.

Use semantic names such as:

```text
bg-rimv-background
bg-rimv-surface
text-rimv-primary
text-rimv-secondary
text-rimv-live
text-rimv-ai
```

Avoid component code full of raw hex values.

## 3. Reuse patterns

If the same utility combination appears repeatedly, consider:

- a component helper;
- a shared class constant;
- a CSS component layer;
- a small reusable DOM builder.

Do not abstract a class list used only once.

## 4. Arbitrary values

Use arbitrary values only when:

- no design token fits;
- the value has a clear design reason;
- reuse is unlikely.

Do not use arbitrary values as a shortcut around the design system.

## 5. Responsive classes

Keep responsive class order predictable.

Prefer simple patterns:

```text
base → md → lg
```

Avoid multiple overlapping breakpoint rules that make behavior difficult to understand.

## 6. Dark mode

Use semantic variables so dark mode changes values centrally.

Avoid duplicate full light/dark class lists on every component.

## 7. Focus

Use explicit focus-visible styles.

Example concept:

```text
focus-visible:ring
focus-visible:outline-none
```

only if the replacement focus ring remains sufficiently visible.

## 8. Component states

Each interactive component should support relevant states:

- hover;
- focus-visible;
- active;
- disabled;
- loading;
- selected;
- error.

## 9. Class ordering

Follow the project's formatter/linting conventions if present.

Do not spend engineering time manually rearranging classes purely for preference unless consistency is materially improved.

## 10. Avoid CSS duplication

Before adding new CSS:

- search for an existing token;
- search for an existing component pattern;
- reuse established state classes.
