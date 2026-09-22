---
name: typescript-vite-tailwind
description: "Use for apps/web TypeScript, plain DOM UI, Vite, Tailwind CSS, PostCSS, responsive layout and browser-engine integration. Do not assume React/Vue/Svelte."
---

# RimV Frontend: TypeScript + Vite + Tailwind

## Verified package assumptions from user-supplied manifest
`apps/web` uses TypeScript 5.x, Vite 6, Tailwind 3, PostCSS/Autoprefixer and Node's built-in test runner. No React/Vue/Svelte runtime is declared. Re-read the real manifest if it may have changed.

## Locate and implement
1. Start in `apps/web/src`; find the actual component/render path, styles and nearby test.
2. Define the UI state machine before updating DOM: idle, loading, live, partial/final transcript, empty, error, cancelled as applicable.
3. Separate transport/API adaptation from rendering. Keep clear typed data contracts with the engine.
4. Reuse existing DOM helpers, Tailwind conventions and design tokens. For visual work consult `.aiassistance/design/RIMV_DESIGN_SYSTEM.md` when present.
5. Write semantic HTML, use real buttons/inputs, accessible names and keyboard handling.
6. Remove listeners, timers and subscriptions when views are replaced or sessions terminate.
7. Reproduce and test error states including stale async responses and offline engine behavior.

## Styling and behavior
- Prefer shared tokens for colors, radii, spacing and typography instead of repeated arbitrary Tailwind literals.
- Preserve responsive content hierarchy; do not shrink the desktop UI into a miniature phone layout.
- Avoid storing backend state in several unrelated frontend flags; derive presentational state from a single source where feasible.
- Do not introduce a component framework, state library, or icon dependency without explicit project justification.

## Available npm commands
```bash
cd apps/web
npm run typecheck  # tsc --noEmit
npm run test       # node --test tests/*.test.mjs
npm run build      # typecheck then Vite build
npm run dev        # local development server
```
`npm run lint` currently also runs `tsc --noEmit`; it is **not** an ESLint/static style analysis command unless the manifest changes. Avoid claiming lint rules were enforced when only type checking ran.

## Exit
Build/typecheck and relevant tests pass; focus and keyboard behavior work; loading/error/cancellation states are explicit; no needless package was introduced.
