# Platform-Specific Exceptions and Decisions

A deliberate difference is acceptable when it improves native usability, accessibility, technical correctness, or reflects missing platform capability. **Silent inconsistency is not acceptable.**

## Allowed reasons

- OS/browser permission restrictions.
- Native UI convention (for example share sheet vs file dialog).
- Accessibility requirement (contrast, motion, scaling, keyboard vs touch).
- Real hardware/backend capability difference.
- Distribution/sandbox/display-server limitation.
- Approved product-scope difference.

A designer's personal preference, ease of copying another platform's screenshot, or a missing token adapter is not by itself a reason to break RimV semantics.

## How to request and approve a deviation

1. Identify the shared requirement in `CONSISTENCY_CONTRACT.md` or canonical 4A design system.
2. State the target platform(s), affected feature, actual constraint and supporting evidence.
3. Compare the native approach to strict visual parity.
4. Preserve recording/privacy truth, shared terminology, accessibility and semantic meaning.
5. Define acceptance criteria and platform-specific validation.
6. Record a decision using `templates/DESIGN_EXCEPTION.md` if the deviation will be maintained long-term.
7. Update the platform guide only after the decision is approved; do not alter the canonical design tokens to solve one platform's issue.

## Cases requiring extra scrutiny

- Remapping `live`, `ai`, `warning`, `error` to different meanings: **not allowed**.
- Removing accessible names or focus visuals to match aesthetics: **not allowed**.
- Hiding Stop while recording on one platform: **not allowed** unless an alternative persistent and usable stop mechanism is documented and validated.
- Automatic audio upload on one target but local-only on another: a product/privacy decision, not just a UI exception; escalate to architecture/security and display true behavior.
- Showing feature controls on an unsupported target: only if disabled state and recovery/explanation are clear, or hide them if discoverability is not needed.

## Example approved-type difference

`Export transcript` can open a native share sheet on iOS, a save dialog on Windows/Linux, or initiate a browser download on Web. Preserve output format choices and user understanding where each implementation supports them. Record any feature differences (for example format not available) separately from UI differences.
