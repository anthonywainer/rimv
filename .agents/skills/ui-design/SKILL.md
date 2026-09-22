---
name: ui-design
description: "Use when building or modifying RimV UI screens, components or states; apply the shared visual rules and only the components relevant to the task."
---

# UI Design Implementation

## Goal
Apply the shared RimV visual identity without loading every platform guide or performing unrelated redesigns.

## Source of truth
`.aiassistance/design/RIMV_DESIGN_SYSTEM.md` and `.aiassistance/design/tokens/rimv.tokens.json`.

## Workflow

1. Identify the target component, screen, platform and current state.
2. Inspect the existing implementation and existing local tokens/component patterns; do not replace an approved design by assumption.
3. Read the **relevant sections** of the design system and one component file under `design/components/` if useful.
4. For platform-specific details, consult the corresponding Part 4B platform file **once installed**; otherwise preserve the codebase's established native patterns.
5. Define default/hover/focus/pressed/busy/disabled or empty/loading/error states where applicable.
6. Use semantic color tokens for light and dark themes; check meaningful-text contrast.
7. Implement the smallest coherent UI change, preserving native interaction and RimV terminology.
8. Verify narrow/wide layout, zoom/text scaling, keyboard/touch and reduced motion as relevant.
9. Report incomplete platform/accessibility testing briefly.

## Boundaries

- This is a skill (procedure), not an additional always-running agent.
- The UI/UX specialist owns substantial user-flow design; this skill helps apply the design consistently.
- Detailed accessibility checks are covered by the existing `.aiassistance/skills/accessibility/SKILL.md`.
- Nielsen's ten-heuristics audit will be Part 4C.
- Do not add React, Vue or another frontend framework solely to adopt the design tokens.

## Completion criteria
Consistent semantic styles, complete relevant interaction states, and no regressions in basic usability.
