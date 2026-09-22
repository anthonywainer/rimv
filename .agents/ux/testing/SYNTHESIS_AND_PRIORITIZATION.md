# From session notes to validated UX improvements

## 1. Clean and preserve evidence

After each session, write a brief observation summary while details are fresh. Preserve the raw notes securely; use coded IDs in the shared synthesis. Mark environmental failures, partial tasks, unknown outcomes and any facilitator assistance.

## 2. Cluster by *root problem*, not heuristic count

Group events that reflect the same underlying problem across participants or platforms. Do not create separate issues for every screenshot or every heuristic a single root cause touches. Keep evidence pointers to tasks/session IDs.

## 3. Write each finding

Use `.aiassistance/ux/testing/templates/FINDING.md`:

- actual observable problem;
- environment/build and relevant platform;
- affected user goal;
- evidence and sample/denominator;
- impact and recovery;
- primary heuristic from Part 4C.1 where applicable;
- accessibility requirement, if independently checked;
- smallest testable recommendation;
- explicit verification task and owner.

Do not assert participant motives or fabricate direct quotes.

## 4. Severity versus delivery priority

**Severity** measures user impact, using the Part 4C.1 0–4 scale: 0 non-issue, 1 minor, 2 moderate, 3 major, 4 blocking/critical. Assign based on observed impact, recurrence, recovery and privacy/safety exposure, not visual dislike.

**Priority** is a separate planning decision incorporating severity, affected flow, product scope, dependencies and implementation effort. Do not quietly lower the severity of a rare privacy-critical recording issue because few participants encountered it. Document disagreements.

## 5. Compare cross-platform behavior

Separate shared semantic failures (e.g., ambiguous recording state) from platform-specific conventions (e.g., Windows Narrator announcement vs macOS VoiceOver). Use `.aiassistance/design/cross-platform/CONSISTENCY_CONTRACT.md` and the matching platform guide.

## 6. Verify after implementation

Write a verification criterion **before** coding the fix. Reproduce the original condition, test regression, run appropriate accessibility checks, and where meaningful retest with representative users. An implemented fix is **not** the same as a verified usability improvement.

## 7. Report limitations

State where evidence is thin: sample composition, untested platforms, different models/OS versions, remote audio latency, missing recording, or prototype restrictions. Use `not tested`, `inconclusive`, and `blocked` rather than treating lack of evidence as a pass.
