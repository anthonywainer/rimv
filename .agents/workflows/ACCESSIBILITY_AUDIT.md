# Workflow — Accessibility Audit

Use the `accessibility-audit` skill for significant reviews; this workflow describes its deliverable.

## 1. Define audited surface

Platform, build, screen/flow, user task, browser/toolkit and relevant assistive technology.

## 2. Inspect

- actual code/DOM/accessibility tree or implemented app;
- primary keyboard/touch path;
- focus changes;
- names, roles, values and status updates;
- error/permission/model/download paths;
- color/contrast, scaling and reduced motion.

Do not assert behavior that cannot be observed from available evidence.

## 3. Verify

For web, map findings to applicable WCAG 2.2 A/AA criteria. For native UI, cite the relevant platform accessibility expectation rather than pretending WCAG is an automatic native-app certification.

Test with a relevant screen reader when available. A successful automated scan does not constitute complete verification.

## 4. Report

Use `.aiassistance/accessibility/templates/ACCESSIBILITY_FINDING.md` for each meaningful issue. Report tested vs not-tested platforms. Group overlapping findings by root cause.

## 5. Remediate and regress

Fix blockers to task completion first. Re-test affected flow, keyboard order and announcements, then relevant visual modes. Avoid cosmetic-only rewrites that don't solve the barrier.

## Concise output

```text
Scope: <platform, build, flow>
Verified with: <tools/manual actions>
Findings: <count, important issues>
Fixed: <if applicable>
Not tested: <platform/assistive technology>
```
