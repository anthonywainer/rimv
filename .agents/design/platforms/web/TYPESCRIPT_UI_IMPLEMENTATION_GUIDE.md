# TypeScript UI Implementation Guide for RimV

## Purpose

Keep RimV Web UI maintainable without assuming a frontend framework.

## 1. Separate responsibilities

Prefer separation between:

- domain state;
- engine/API communication;
- DOM rendering;
- event binding;
- formatting.

Avoid one large module that performs all responsibilities.

## 2. Explicit state

For complex workflows, represent state explicitly.

Example:

```ts
type CaptureState =
  | { kind: "idle" }
  | { kind: "preparing" }
  | { kind: "listening" }
  | { kind: "transcribing" }
  | { kind: "error"; message: string };
```

Prefer this over many booleans that can become inconsistent.

## 3. DOM queries

Query elements intentionally.

Fail clearly when a required root element is missing.

Avoid repeated global DOM queries inside hot update paths.

## 4. Event listeners

Keep listener lifetime explicit.

When views/components are disposable, remove listeners or use abort signals where appropriate.

## 5. Async operations

For asynchronous UI work:

- show pending state;
- handle errors;
- avoid race conditions;
- ignore stale responses when necessary;
- cancel superseded work where supported.

## 6. AbortController

Use `AbortController` when browser APIs and task lifecycle make cancellation useful.

Do not keep obsolete requests running without reason.

## 7. Rendering

Update only the DOM that needs to change.

Avoid replacing entire large containers for small state updates.

## 8. HTML generation

Prefer DOM APIs or safe template patterns.

Do not inject untrusted text with `innerHTML`.

Use `textContent` for user/engine-generated text unless sanitized markup is explicitly required.

## 9. Accessibility state

When UI state changes:

- update `aria-*` attributes where relevant;
- preserve focus;
- announce important asynchronous updates appropriately;
- avoid overly chatty live regions.

## 10. Type safety

Prefer narrow types.

Avoid `any`.

Parse external/engine data defensively at boundaries.

## 11. Errors

Separate:

- diagnostic error;
- user-facing message.

The browser console may contain technical detail while the UI presents a concise recovery path.

## 12. Testing

Current project scripts support:

- `npm run typecheck`
- `npm run test`
- `npm run build`

Prefer the smallest relevant validation first.

## 13. Dependency discipline

Do not add packages for functionality available cleanly through:

- browser APIs;
- TypeScript;
- existing dependencies.

Every new runtime dependency increases maintenance and bundle cost.
