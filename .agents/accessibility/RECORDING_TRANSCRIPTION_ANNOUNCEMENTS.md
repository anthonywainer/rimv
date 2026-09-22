# Accessible Status and Live Transcript Behavior

RimV's dynamic audio states are the highest-priority accessibility interactions. The visual interface and accessibility tree must agree.

## State announcement matrix

| State / event | Visible label or feedback | Accessible behavior | Focus policy |
|---|---|---|---|
| Idle | `Ready to listen` / `Start listening` | Named Start action, idle state available | Remain on user's control |
| Permission requested | Explain microphone permission | Explain context before OS/browser dialog | Let native dialog manage focus |
| Permission denied | `Microphone access is off` + recovery | Announce one actionable error; expose Settings/retry | Focus error heading only if blocking view replaces current content |
| Preparing | `Preparing microphone…` | One status change; avoid repeated chatter | Keep focus on initiating control or sensible successor |
| Listening/recording | `Recording` + persistent Stop | One confirmed start announcement; Stop reachable by standard navigation | Never steal focus just because recording began |
| Paused | `Paused` + Resume/Stop | State changes once | Keep control focus stable |
| Stopping | `Stopping…` | Distinguish requested stop from confirmed completion | Preserve context |
| Transcribing | `Transcribing…` | One status; optional progress when measurable | Transcript remains discoverable |
| Partial text arrives | Visually provisional, appropriately labeled | **Do not** announce each word/token automatically | Do not reset selection or scroll |
| Final transcript arrives | Stable final text | Brief `Transcript ready` status; expose output on request | Don't jump focus unless user explicitly requested it |
| Download progressing | Name/model + numeric progress if known | Accessible progress value; announce meaningful milestones, not every % | Keep cancel button operable |
| Operation completed | `Download complete` / `Transcript ready` | Announce once | Retain user's location |
| Recoverable error | Short problem + remedy | Assertive only when urgent; otherwise polite; avoid repeated announcements | Move focus only if needed for recovery |
| Device unplugged while live | `Microphone disconnected` + Stop/reselect | Announce interruption once; reflect actual capture state | Make recovery controls accessible |

## Web implementation pattern

Use native elements first. A *stable* status region can be updated with a short string, instead of replacing the entire subtree. Use `role="status"` (polite) for normal meaningful changes. Reserve `role="alert"` for urgent conditions and avoid combining competing `aria-live` configurations redundantly.

```html
<p id="capture-status" role="status" aria-atomic="true">Ready to listen</p>
<button type="button" id="capture-start">Start listening</button>
<button type="button" id="capture-stop" hidden>Stop recording</button>
<section aria-labelledby="transcript-heading">
  <h2 id="transcript-heading">Transcript</h2>
  <div id="transcript-text" aria-live="off"></div>
</section>
```

Update status sparingly. Do not repeatedly set entire transcript `textContent` if it invalidates selection/reading position. When async errors occur, present a persistent error message associated with a useful action.

## Announcement frequency

- Announce **state transitions**, not audio waveform frame changes or elapsed-time ticks.
- Use meaningful progress milestones when needed; exposing an accessible progress *value* is not the same as announcing every update.
- Avoid a flood of partial transcript changes.
- If the user opts into continuous transcript reading, offer pause/stop of announcements and document the behavior.
- If multiple events occur simultaneously, prioritize interruption, stop and recoverable error over cosmetic completion messages.

## Native mapping

- Apple: accessibility label/value, announcement APIs only when warranted; avoid repeated VoiceOver interruption.
- Windows: UI Automation name/state/value and appropriate live-region patterns; test Narrator behavior.
- Linux: toolkit accessibility events and AT-SPI announcements; test Orca where supported.
- Web: semantic controls and sparing polite status/live regions; test VoiceOver + Safari and NVDA + Firefox/Chrome where possible.

## Privacy-sensitive content

A transcript may contain personal information. Don't automatically read out transcript content in notifications or generic live status announcements. Screen readers should be able to reach and read the transcript intentionally.
