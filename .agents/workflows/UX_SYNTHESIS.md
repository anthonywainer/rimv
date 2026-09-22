# Workflow — UX evidence to prioritized changes

Input: coded session notes, actual task outcomes, build/platform and approved evidence references.

1. Confirm denominators and missing tasks; separate pilot sessions or mixed versions.
2. Extract observation statements with time/task references. Mark direct participant feedback separately from researcher interpretation.
3. Cluster by root issue across sessions; do not double-count a single problem under several heuristics.
4. Create one `ux/testing/templates/FINDING.md` per root issue with Part 4C.1 severity and an independently observable verification condition.
5. Separate severity from delivery priority and record privacy-critical risks explicitly.
6. Check overlapping accessibility findings in Part 4C.2, without calling user testing a conformance test.
7. Plan the smallest reasonable change; assign owner and retest environment.
8. Produce `ux/testing/templates/UX_REPORT.md` for the scoped decision, stating uncertainty.

A completed code change closes implementation work, **not** necessarily the usability finding: close the finding after the stated verification.
