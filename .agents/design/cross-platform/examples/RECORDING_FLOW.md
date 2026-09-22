# Worked Example — Recording and Transcription Across Platforms

**Illustrative UX contract only.** Verify the actual RimV engine API and platform feature set before implementation. This example demonstrates reuse of shared semantics with native UI.

## Scenario

A user chooses an audio input, starts listening, sees partial transcript updates, stops capture, waits for final transcription and copies/exports the result.

## Shared flow

```text
Idle
 ├─ no permission → Permission required → recovery → Idle
 ├─ no model/device → Unavailable → select/download → Idle
 └─ Start → Preparing
              ├─ start rejected → Recoverable failure → Retry
              └─ engine confirms → Listening
                                       ├─ interruption → Recoverable failure
                                       ├─ partial text → Partial transcript
                                       └─ Stop → Stopping → [Transcribing if needed] → Completed
```

**Caveat:** capture and transcript processing may run concurrently; model them as independent states where the actual engine requires it.

## Visual invariant

In `Listening`, each supported UI uses the canonical `live` token *or an accessible native adaptation* and persistent `Listening` text/status. No platform may show that indicator merely because the Start button was pressed. Primary Stop stays accessible.

AI-generated insights, if implemented, appear in a distinct clearly labeled region after/alongside the source transcript. The purple `ai` semantic is not used to mark recording-active.

## Native patterns

| Target | Start/stop location | Permission recovery | Final text action |
|---|---|---|---|
| macOS | Main toolbar or focused status/menu-bar surface if implemented | System Settings guidance | Native copy/save/share |
| iOS | Large primary touch action; status persists during relevant flow | OS settings guidance on denial | System share sheet |
| Windows | Visible primary action; optional tray/keyboard shortcut if implemented | Windows privacy guidance | Copy/save dialog |
| Linux | Toolkit-native primary action; tray never required | Explain device/audio stack or sandbox access | Copy/portal/native save |
| Web | Semantic button in primary view | Browser site permissions explanation | Copy/download where supported |

Avoid assuming a platform supports background capture, global shortcuts, notifications, or every export format without verifying product implementation.

## Failure exercise

1. User presses Start.
2. UI shows `Preparing microphone…`.
3. Engine reports microphone unavailable (for example unplugged device).
4. UI clears preparing state; shows `Microphone unavailable. Choose another input or check access.`
5. Available next action: device picker or Retry.
6. Screen-reader announcement reports one meaningful error; UI never briefly claims `Listening` after the failure.

## Verification examples

- Mock/simulate delayed engine start; ensure premature `Listening` never appears.
- Simulate immediate stop after start, if the engine permits it.
- Test device disconnect while listening; verify live indicator disappears as soon as actual capture ends.
- Feed a long partial/final transcript; preserve scroll/selection and distinguish AI output.
- Use keyboard or assistive input to Stop without entering advanced settings.
