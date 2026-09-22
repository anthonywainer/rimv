# Skill: Usability Heuristics Review

## Use when

The task asks to:
- review usability
- audit a screen or flow
- identify UX problems
- improve interaction clarity
- evaluate an existing UI
- apply Nielsen usability heuristics

## Do not use when

The task is purely:
- visual styling
- code cleanup
- backend implementation
- performance work
- accessibility-only review

## Required references

Read only what is relevant:
1. `.aiassistance/ux/usability-heuristics/RIMV_USABILITY_HEURISTICS.md`
2. `.aiassistance/ux/usability-heuristics/RIMV_FLOW_REVIEW_GUIDE.md` for flows
3. `.aiassistance/design/RIMV_DESIGN_SYSTEM.md` when UI context matters
4. target platform guidance

## Procedure

### 1. Define scope
Identify platform, screen/flow, primary user goal, and success condition.

### 2. Observe before judging
Record concrete behavior:
- what user sees
- what action they take
- what feedback appears
- what failure/recovery exists

Do not treat personal visual preference as a usability violation.

### 3. Apply H1–H10
Only create findings supported by observable evidence.

### 4. Consolidate root causes
If one issue touches several heuristics, prefer one finding with the primary heuristic.

### 5. Assign severity
Use:
- 1 minor
- 2 moderate
- 3 major
- 4 critical/blocking

Consider frequency, impact, persistence, recoverability, and privacy/safety.

### 6. Recommend the smallest effective fix
Recommendations must be concrete, platform-appropriate, consistent with RimV, and proportionate.

### 7. Output format

```text
Finding:
Heuristic:
Severity:
Evidence:
Impact:
Recommendation:
```

Sort multiple findings by severity descending.

### 8. Completion
If no meaningful issue is found, say so. Do not invent findings to fill every category.
