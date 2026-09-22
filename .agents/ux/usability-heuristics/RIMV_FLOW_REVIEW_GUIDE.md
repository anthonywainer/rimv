# RimV Flow Review Guide

## 1. Define the flow

Example:

```text
Open RimV
→ choose microphone
→ start listening
→ speak
→ receive transcript
→ stop
→ copy/export
```

## 2. Define success

Example:
`The user records speech and obtains a readable transcript without technical knowledge.`

## 3. Walk each state

At each step ask:
- What does the user see?
- What do they think is happening?
- What action is available?
- What happens next?
- What can fail?
- Can they recover?

## 4. Apply heuristics

Do not mechanically create one finding for every heuristic.

Record findings only when there is a concrete usability issue.

## 5. Check alternate paths

When relevant:
- permission denied
- microphone disconnected
- no model
- download failure
- empty transcript
- long transcript
- cancellation
- interruption

## 6. Prioritize

Fix critical and major problems before cosmetic issues.

## 7. Avoid redesign inflation

Prefer the smallest effective change instead of turning every audit into a full redesign.
