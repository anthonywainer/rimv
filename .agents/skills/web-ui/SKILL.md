# Skill: RimV Web UI Development

## Use when

A task changes:

- `apps/web`;
- browser UI;
- TypeScript UI state;
- Tailwind styles;
- responsive behavior;
- browser microphone/permission UX;
- web accessibility.

## Required references

Read only those relevant to the task:

1. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`
2. `.aiassistance/design/platforms/web/WEB_PLATFORM_GUIDELINES.md`
3. `TAILWIND_IMPLEMENTATION_GUIDE.md` for styling work
4. `TYPESCRIPT_UI_IMPLEMENTATION_GUIDE.md` for frontend logic
5. `WEB_ACCESSIBILITY_CHECKLIST.md` for accessibility-sensitive work
6. `WEB_UI_REVIEW_CHECKLIST.md` for reviews

## Procedure

### 1. Identify the existing implementation

Before editing:

- locate the relevant TypeScript/HTML/CSS;
- search for existing component/pattern;
- identify current state model;
- identify relevant tests.

Do not assume a framework.

### 2. Identify states

For interactive features, determine relevant:

- idle;
- pending;
- active;
- success;
- empty;
- permission denied;
- error;
- retry.

### 3. Apply design tokens

Use existing RimV semantic tokens.

Do not invent one-off visual values if a token exists.

### 4. Preserve browser semantics

Prefer native HTML behavior.

Do not replace accessible controls with generic clickable containers.

### 5. Check responsive behavior

Verify that:

- primary content remains visible;
- secondary content collapses appropriately;
- text is not clipped;
- browser zoom remains usable.

### 6. Check accessibility

At minimum:

- keyboard;
- focus;
- accessible labels;
- contrast;
- reduced motion;
- screen-reader-relevant state changes.

### 7. Check TypeScript behavior

Avoid:

- global mutable state without need;
- race conditions;
- stale async updates;
- unsafe HTML insertion;
- unnecessary dependencies.

### 8. Validate

Prefer:

1. targeted test;
2. `npm run typecheck`;
3. `npm run test`;
4. `npm run build` when bundling/build behavior is affected.

## Completion

Report only:

- key change;
- validation performed;
- unresolved browser/accessibility issue if any.
