# Default Task Execution Workflow

Use this workflow for most development tasks unless a more specific workflow exists.

## Phase 1 — Understand

1. Identify the requested outcome.
2. Identify explicitly mentioned files, symbols, errors, or modules.
3. Determine the primary technical area.
4. Determine whether specialist guidance is required.
5. Note acceptance criteria from the task.

If the request is materially ambiguous and multiple implementations would have different behavior, clarify before making a large change.

## Phase 2 — Locate

Use this search order:

1. named file;
2. named symbol;
3. project map;
4. direct references/callers;
5. relevant tests;
6. broader search.

Do not begin with a repository-wide file dump.

## Phase 3 — Inspect

Read only enough code to answer:

- Where does the behavior live?
- What contract must remain compatible?
- What existing pattern should be followed?
- What test proves the behavior?

For UI work also determine:

- platform;
- design-system rules;
- existing component pattern;
- accessibility impact.

## Phase 4 — Plan

For non-trivial tasks, form a short internal implementation plan:

- files likely to change;
- contract impact;
- test/validation strategy;
- notable risk.

Do not create large planning documents for small fixes.

## Phase 5 — Implement

Rules:

- make the smallest coherent change;
- reuse existing abstractions;
- preserve naming and architectural conventions;
- avoid opportunistic refactors;
- avoid new dependencies unless justified;
- keep platform-specific behavior isolated where appropriate.

## Phase 6 — Validate

Run the narrowest meaningful checks first.

Examples:

### Rust
- relevant unit test;
- `cargo test -p <package>`;
- `cargo check -p <package>`;
- workspace-wide validation only when needed.

### Web
- targeted test where available;
- `npm run typecheck`;
- `npm run test`;
- `npm run build` when build behavior is affected.

### Platform applications
Use the platform-specific build/test workflow defined by its specialist guidance.

## Phase 7 — Diagnose failures

When a validation step fails:

1. determine whether the failure was caused by the change;
2. inspect the smallest relevant context;
3. fix the cause rather than masking the symptom;
4. rerun the failed check;
5. expand validation only after the targeted check passes.

## Phase 8 — Complete

Default response format:

```text
Done.
- <important result if useful>
- Tests: <passed check>

Issues:
- <only if unresolved>
```

For very small successful changes, one or two lines are sufficient.

Do not generate a long summary unless requested.
