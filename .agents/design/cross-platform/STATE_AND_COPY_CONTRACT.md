# Shared UI State, Microcopy and Privacy Contract

This document defines **observable UX semantics**, not Rust/IPC/wire-protocol structures. Do not assume these exact names exist in RimV source today. Confirm actual runtime states before wiring UI.

## 1. Recommended capture state model

| User-visible state | Meaning | Required feedback | Allowed main action |
|---|---|---|---|
| Idle | Capture is stopped and UI can initiate it | Ready state, chosen input if relevant | Start |
| Permission required | System/browser permission prevents starting | Explain access and recovery | Request access / Open settings as applicable |
| Preparing | Requested start is in progress; engine not yet confirmed active | Preparing indicator | Cancel if supported |
| Listening | Capture has been confirmed active | Persistent live icon + label; device/time if useful | Stop |
| Stopping | User requested stop; shutdown not complete | Stop-in-progress indicator | Prevent duplicate stop; allow safe recovery |
| Transcribing | Audio has been captured and transcript is processing | Progress/status; allow safe cancel if supported | Cancel where supported |
| Completed | Transcript is finalized | Final state; selectable content | Copy/export/new capture as supported |
| Recoverable failure | Operation failed with actionable recovery | Error + next action + diagnostic detail elsewhere | Retry / select alternative |
| Unavailable | Device/model/backend is missing or unsupported | Explanation without fake action | Install/select/open supported alternative |

If the runtime supports overlapping capture and transcription, **don't force a single exclusive enum onto it**. Represent the independent dimensions (capture status + transcript processing status + model readiness) and present an honest composed UI. Avoid impossible UI combinations such as `Listening` after engine shutdown.

## 2. State changes and event truth

- Clicking Start means `Preparing`, not `Listening`, until engine confirms successful start.
- Clicking Stop means `Stopping`, not `Idle`, until capture ends.
- An interruption/device loss immediately invalidates the live indicator once the runtime reports capture stopped.
- Partial transcript is not committed final text; avoid implying certainty or retaining stale partials after finalization unless product semantics require it.
- Model `Downloaded`, `Verified`, `Loaded` and `Ready` can be distinct; don't conflate them.
- If operation progress cannot be quantified, use an indeterminate indicator, not a made-up percentage.

## 3. Shared terminology and sample strings

The following English strings are recommended product-copy conventions, not mandatory verbatim localized text. Keep meaning equivalent across languages.

| Concept | Preferred short text | Avoid |
|---|---|---|
| Start capture | Start listening | Launch engine, Capture ON |
| Stop capture | Stop listening | Turn off processing (ambiguous) |
| Start pending | Preparing microphone… | Recording… before confirmation |
| Capture active | Listening | Ready (ambiguous) |
| Engine processing | Transcribing… | Thinking… (ambiguous) |
| Final result | Transcript ready | Finished? |
| Input unavailable | Microphone unavailable. Choose another input or check access. | HRESULT… / stream IO failed |
| Permission denied | Microphone access is off. Enable it in your device or browser settings. | Permission error 403 |
| Model unavailable | Model isn't ready. Download or choose an available model. | Model invalid (without action) |
| Network-only operation | This action requires a network connection. | Network error (without action) |
| Explicit AI content | AI summary | Transcript, if generated interpretation |

Platform labels can adapt (`Settings` on Apple, `Settings` on Windows, browser site settings on Web), but describe equivalent recovery.

## 4. Confirmation and cancellation

- Immediate reversible choices do not require repeated confirmations.
- Delete/clear/export-overwrite should communicate consequences and follow platform-native destructive patterns.
- Stop and Cancel have different meanings: stop capture may still process existing audio; cancel transcription may discard incomplete output. Label according to actual behavior.
- Persist input and context through recoverable errors when safe and expected.

## 5. Transcript and AI boundaries

- Distinguish *source speech transcription* from *AI summary/interpretation* with heading, accessible name and semantic styling.
- If partial and final segments are shown together, use a stable visual distinction that does not rely on color alone.
- Preserve user scroll/selection when live text updates; don't unexpectedly yank focus.
- Clearly communicate if text may change before finalization.
- Do not claim transcription accuracy or model confidence not provided by the engine.

## 6. Privacy truthfulness

- Display visible capture status everywhere capture controls/status are available.
- Don't claim "all audio stays on this device" until architecture/network behavior has actually been checked for that feature.
- When platform permissions vary, adapt recovery instructions instead of inventing a uniform permission prompt.
- Explain storage and deletion using implemented behavior; do not invent retention windows.
- Give users relevant information about background recording if it exists, without relying on OS UI alone.

## 7. Notification and assistive announcement

- Announce capture start/stop, important errors, and meaningful completion when appropriate.
- Do not narrate every new transcript token to screen readers; announce meaningful chunks or user-invoked updates.
- Background/system notifications should be reserved for user-relevant completion/failure and obey permission conventions.
- Use native status announcements on Apple/Windows/Linux and suitable ARIA live regions on Web, with non-spammy update cadence.
