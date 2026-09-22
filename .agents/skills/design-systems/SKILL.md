---
name: design-systems
description: "Use when creating/updating RimV semantic color, typography, spacing, component tokens or changing shared UI visual standards across platforms."
---

# Maintain RimV's Design System

## Canonical documents

- `.aiassistance/design/RIMV_DESIGN_SYSTEM.md` — human-readable decisions.
- `.aiassistance/design/tokens/rimv.tokens.json` — machine-readable reference.
- `.aiassistance/design/SOURCE_NOTES.md` — Hamu provenance and deliberate adaptations.

## Workflow

1. Identify whether the request changes shared identity, one component, or only a platform-native convention.
2. For shared changes, update the canonical design system **first**; amend token data in the same change.
3. Check light/dark semantic pairs, focus, meaningful-text contrast and full control state matrix.
4. Update relevant `design/components/` guidance only if component behavior actually changes.
5. Update platform mapping when Part 4B is installed; do not force identical native controls.
6. Migrate existing app code **only if explicitly requested** and after checking current implementation.
7. Document intentional deviations and Hamu-versus-RimV decisions in `SOURCE_NOTES.md`.
8. Review `.aiassistance/design/DESIGN_QA.md` on affected areas.

## Anti-patterns

- Copying palette tables into every agent/skill.
- Scattering raw hex values throughout CSS/Swift/Windows/Linux code.
- Treating token JSON as automatically wired into app code when no generator/import exists.
- Claiming Hamu's multiple historical palettes are identical.
- Creating a second version of `RIMV_DESIGN_SYSTEM.md` in `resources/`.

## Completion
Human and token references agree; affected components have complete states; meaningful colors and platform semantics remain coherent.
