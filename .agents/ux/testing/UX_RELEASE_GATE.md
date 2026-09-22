# UX release review and exit criteria

Use this for significant changes to capture, permissions, models, transcription, export and cross-platform navigation; a one-line CSS fix rarely needs the full gate.

## Evidence gate

- [ ] Affected user goals and platforms are identified.
- [ ] Relevant heuristic findings from Part 4C.1 are triaged.
- [ ] Accessibility verification from Part 4C.2 is recorded separately.
- [ ] Test setup, build/version, sample and limitations are recorded.
- [ ] Safety/privacy-sensitive capture behavior has direct validation.
- [ ] No “passed” status is assigned to an untested environment.

## Critical flows

- [ ] Recording state is discernible with and without color.
- [ ] Stop capture actually stops audio, including interruption/recovery paths in scope.
- [ ] Permissions denied and device lost have recoverable UI.
- [ ] Model readiness and downloads communicate progress and failure.
- [ ] Transcript partial/final distinction is clear where applicable.
- [ ] Copy/export preserves intended final text.
- [ ] Actual data handling matches user-facing privacy language.

## Platform review

Record the exact platform(s) verified. Follow the appropriate guides under `design/platforms/` and cross-platform consistency documents. **Not tested** is a valid, honest status for unsupported or unavailable environments.

## Decision record

Use `PASS`, `PASS WITH KNOWN ISSUES`, `BLOCKED`, or `NOT EVALUATED` *for the defined release scope*, with owner, date, unresolved findings and mitigation. This is an operational release gate, not a claim that the entire product is usable or accessible. Critical/major findings require explicit owner acknowledgement and a documented resolution or release decision; do not silently waive them.

## After release

Monitor relevant issue reports; confirm shipped behavior and perform a targeted follow-up if the tested build differs from the release build.
