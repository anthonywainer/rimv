# RimV Usability Heuristics Guide

## H1 — Visibility of system status

Users should always know whether RimV is idle, requesting permission, preparing, listening, recording, transcribing, processing, downloading a model, ready, or failed.

Good:
- persistent recording state
- measurable progress
- clear loading text
- partial/final transcript distinction

Bad:
- color-only status
- invisible microphone activity
- long initialization with no feedback

Review:
- Can the user tell what is happening?
- Is progress shown when useful?
- Is there immediate action feedback?

## H2 — Match between system and the real world

Use user language rather than implementation language.

Prefer:
- Microphone
- Start listening
- Stop recording
- Transcript
- Download model
- Audio input

Avoid presenting raw terms such as buffer underrun, FFI failure, IPC failure, ONNX provider, or backend error as the main UI message.

## H3 — User control and freedom

Users should be able to stop, cancel, dismiss, retry, or recover when technically safe.

RimV examples:
- Stop capture
- Cancel model download
- Retry transcription
- Close a dialog
- Recover after permission denial

Avoid modal traps and hidden stop actions.

## H4 — Consistency and standards

Maintain consistency at three levels:
1. RimV product semantics
2. platform-native conventions
3. feature-level behavior

Examples:
- same terminology for start/stop
- consistent error semantics
- same AI semantic treatment
- native macOS/Windows/Linux/Web behavior

Do not force one platform's UI conventions onto another.

## H5 — Error prevention

Prevent failures where practical.

Examples:
- prevent transcription without a model
- prevent capture with no valid input
- confirm destructive loss
- block duplicate submissions
- validate incompatible configuration early

Do not over-disable controls without explaining why.

## H6 — Recognition rather than recall

Keep important context visible.

Show:
- current microphone
- selected model
- selected language
- current live state
- useful recent/default choices

Avoid cryptic icons and hidden essential commands.

## H7 — Flexibility and efficiency of use

Support both new and experienced users.

New users:
- clear defaults
- guided permission flow
- obvious primary action

Experienced users:
- shortcuts
- quick switching
- efficient copy/export
- reusable preferences

Keep advanced options secondary.

## H8 — Aesthetic and minimalist design

Typical RimV visual priority:
1. capture state
2. transcript/content
3. primary action
4. relevant AI result
5. secondary controls
6. diagnostics

Avoid decorative overload, duplicated status, competing primary actions, and technical diagnostics in the main flow.

## H9 — Help users recognize, diagnose, and recover from errors

A useful error answers:
1. What happened?
2. What can the user do?

Good:
`Microphone unavailable. Select another input device or check microphone access in system settings.`

Poor:
`Error -9986`

Possible recovery actions:
- Retry
- Open Settings
- Select Device
- Download Model
- Reconnect
- View Details

## H10 — Help and documentation

Common tasks should be understandable without documentation, while help should exist for advanced or unfamiliar tasks.

Useful help:
- first-use guidance
- microphone permissions
- model information
- shortcuts
- export behavior
- privacy explanation
- troubleshooting
- diagnostics

Do not make basic start/stop operation depend on reading documentation.

# High-risk RimV flows

Prioritize heuristic review for:
- microphone permissions
- recording/listening
- model setup and download
- live transcription
- AI-generated content
- settings

# Severity scale

## 0 — Not a usability problem
No change required.

## 1 — Minor
Small friction with little impact.

## 2 — Moderate
Causes confusion or inefficiency but task completion remains practical.

## 3 — Major
Substantially disrupts task completion or causes repeated failure.

## 4 — Critical / blocking
Prevents task completion or creates serious safety/privacy/data-loss risk.

Consider frequency, impact, persistence, recoverability, and privacy/safety.

# Finding format

```text
Finding: <problem>
Heuristic: H<n> — <name>
Severity: 1–4
Evidence: <observable behavior>
Impact: <why it matters>
Recommendation: <smallest effective fix>
```

Do not count one root problem as multiple findings merely because it touches several heuristics.
