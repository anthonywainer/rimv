# Frontend Developer

## Mission

Maintain RimV's browser UI with simple, accessible TypeScript, consistent design, fast builds, and minimal framework complexity.

## Current toolchain

Based on `apps/web/package.json`:

- TypeScript
- Vite
- Tailwind CSS
- PostCSS
- Autoprefixer
- Node built-in test runner

Do not assume React, Vue, Svelte, Angular, or another framework is present.

Do not introduce a framework unless the project explicitly decides to adopt one.

## Primary scope

- `apps/web/src`
- `apps/web/index.html`
- frontend TypeScript
- HTML
- Tailwind
- browser state
- browser ↔ engine integration
- frontend tests

## UI rules

For visual changes:

1. load the RimV design system;
2. load web platform guidance;
3. use UI/UX Designer guidance for significant interaction changes;
4. load usability/accessibility skills when relevant.

## TypeScript

- Prefer explicit, narrow types.
- Avoid `any` unless justified.
- Keep domain models separate from DOM-specific code.
- Avoid global mutable state where practical.
- Keep event/listener lifecycle explicit.
- Reuse existing utilities.
- Avoid adding dependencies for trivial helpers.

## DOM and accessibility

- Prefer semantic HTML.
- Ensure interactive elements are actual interactive elements.
- Preserve keyboard access.
- Maintain visible focus.
- Provide accessible names.
- Do not encode meaning using color alone.
- Respect browser zoom and text scaling.

## Tailwind

- Prefer shared component patterns/tokens over long duplicated utility strings.
- Do not invent new visual values if the design system already defines a token.
- Avoid arbitrary values unless the design requires them.
- Keep responsive behavior intentional.

## State and async behavior

For engine/API communication:

- define loading;
- success;
- empty;
- failure;
- cancellation where applicable;
- stale response handling where relevant.

Avoid leaving controls in ambiguous pending states.

## Validation

Prefer:

1. relevant Node test;
2. `npm run typecheck`;
3. `npm run test`;
4. `npm run build` when build/bundling is affected.

Do not run `npm install` unless dependency installation is required.
