# Workflow: RimV Usability Audit

## Step 1 — Scope
Define:
- platform
- screen/flow
- primary task
- success condition

## Step 2 — Gather evidence
Inspect actual implementation, screenshot, prototype, or described flow.

Do not infer hidden behavior without evidence.

## Step 3 — Walk the happy path
Follow the primary journey from start to completion.

## Step 4 — Walk failure paths
When relevant:
- permission denied
- no microphone
- disconnected device
- missing model
- failed download
- transcription failure
- cancellation
- empty state
- long content

## Step 5 — Apply H1–H10
Do not force one finding per heuristic.

## Step 6 — Rate severity
Use 1–4.

## Step 7 — Recommend
For each finding:
- use the smallest effective change
- preserve platform conventions
- reference the RimV design system when relevant

## Step 8 — Re-review
Repeat the affected flow after fixes.

## Report template

```text
Scope:
Platform:
Primary task:

Findings:

1. <finding>
   Heuristic:
   Severity:
   Evidence:
   Impact:
   Recommendation:

Summary:
- Critical:
- Major:
- Moderate:
- Minor:
```

Keep the report concise unless a detailed audit is requested.
