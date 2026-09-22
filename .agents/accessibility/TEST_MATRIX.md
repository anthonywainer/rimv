# RimV Accessibility Test Matrix

Run affected rows for every meaningful UI change; do not run all platforms for an unrelated small change. Before release, sample every supported platform and high-risk flow.

## Cross-platform manual matrix

| Scenario | Keyboard/touch | Screen reader | Visual/scaling | Failure path |
|---|---|---|---|---|
| Start capture | Activate by accessible control | Announces actual start once | State text + icon, visible Stop | Permission denied |
| Stop active capture | Reach Stop without pointer | Correct name/state | Stop distinguishable in all themes | Device disconnect during stop |
| Select input device | Navigate and confirm | Reads selected device | Enlarged labels | Device unavailable |
| Model download | Cancel if safe | Meaningful progress | Readable progress at zoom | Disk/network failure |
| Live transcript | Navigate without focus jump | No automatic token flood | Reflow, stable scrolling | Interrupted stream |
| Long transcript | Select/copy/export | Logical text reading | 200/400% zoom or native scaling | Empty/export error |
| AI result | Discover separation from verbatim source | Distinguishable heading/name | Does not rely on purple alone | AI processing failure |
| Settings | Tab/arrow/touch | Current values read | High contrast/large text | Invalid configuration |
| Dialog | Logical focus entry/exit | Proper title and controls | No clipped labels | Cancel/return |

## By platform

| Platform | Primary manual tests | Secondary tests |
|---|---|---|
| Web | Keyboard only, one screen reader/browser, 200/400% zoom, automated analyzer | Forced colors, alternate browser, speech input |
| macOS | VoiceOver, keyboard only, resize, Increase Contrast | Reduced Motion, alternate input device |
| iOS | VoiceOver, Dynamic Type including large sizes, touch targets | Voice Control, external keyboard, interruptions |
| Windows | Narrator, keyboard only, high contrast, 125/150% scaling | Accessibility Insights/UIA Inspect, alternative screen reader |
| Linux | Orca or appropriate reader for chosen toolkit, keyboard, font scaling | Alternate DE, Wayland/X11, sandbox testing |

## Evidence

Each completed manual test records: platform, OS/build, browser/toolkit, screen-reader version if used, test date, task, expected result, actual result, pass/fail, issue reference and any limitation. Do not write `tested` if only a static code review was performed.

## Automation scope

Automated accessibility checkers can detect missing labels, malformed relationships and some color/structure issues. They cannot by themselves verify understandable announcements, correct focus restoration, usable long transcripts, intuitive error recovery or native-platform assistive technology behavior.

## Exit

No known blocker prevents an affected primary task from keyboard/assistive technology use. Unverified platform states are recorded as **not tested**, not presumed passing.
