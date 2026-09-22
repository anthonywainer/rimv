# Cross-Platform Consistency Contract

## 1. Definition

A coherent RimV experience is **behaviorally consistent** and **visually recognizable**, while preserving each platform's native interaction patterns. Consistency does not require pixel-identical screenshots.

Every platform implementation should make the same core questions easy to answer:

1. Is RimV capturing audio right now?
2. Which audio input/model is in use, if exposed by that product?
3. Is the transcript partial, finalized or unavailable?
4. Which content comes directly from recognized speech and which is AI-generated?
5. What can I do next, including stop, cancel, retry, copy and export when supported?
6. What is stored or transmitted, based on the actual implemented behavior?

Do not imply capabilities that a platform has not implemented. Mark unsupported and untested states explicitly in reviews.

## 2. Required invariants

| Invariant | Required outcome | Where defined |
|---|---|---|
| Semantic color | `live`, `ai`, `warning`, `error`, `focus` carry consistent meaning | Part 4A tokens |
| Recording truth | Capture indicator reflects actual engine state, not an optimistic button press | State contract |
| User control | Stop/cancel/retry are identifiable when supported | State contract |
| Source distinction | Finalized transcript vs AI interpretation is never ambiguous | Component parity |
| Terminology | Common concepts use one glossary, localized as needed | State/copy contract |
| Accessibility | Primary flow works via available platform accessibility modes | Part 4C + platform guide |
| Error recovery | User gets a relevant next action, not a raw error dump | State/copy contract |
| Preference semantics | Equivalent theme, input/model, language settings mean the same where implemented | Settings parity |

Shared semantics are requirements; presentation is flexible. Red cannot mean *normal capture* on one platform and *error* on another. If the visual system must adapt for contrast, preserve the meaning through icon/label and accessible name.

## 3. Deliberate native differences

| Area | Apple | Windows | Linux | Web |
|---|---|---|---|---|
| Icons | SF Symbols | Fluent/system | Toolkit/icon theme | One coherent web SVG family |
| Menus | App menu, native menus/sheets | Native menu/commands | Toolkit/menu/portal as applicable | Semantic HTML menus/dialogs |
| Status surface | Menu bar where implemented | Notification area/tray where implemented | Tray is optional by desktop | In-page state, browser constraints |
| Microphone permission | Apple OS permission | Windows privacy permission | Audio-stack/portal/app permissions vary | Browser `getUserMedia` permissions |
| Keyboard | Apple conventions | Windows conventions | Toolkit/desktop conventions | Browser conventions |
| Typography | System SF family | Segoe/system | Toolkit/system | Web tokens + system fallback |
| Windowing | Native windows/sheets | Native window/dialog | Toolkit/compositor-specific | Responsive browser viewport |

**Important:** this table shows design choices **if the capability exists**. It does not mandate a tray, native app or iOS release.

## 4. Precedence on conflict

1. Safety, accurate privacy/recording status, accessibility, platform policy and explicit product requirements.
2. Approved RimV design decisions and shared semantics (`../RIMV_DESIGN_SYSTEM.md`).
3. Host-platform conventions for behavior and native widgets.
4. Local existing component patterns where consistent with the above.

If adapting a shared token or layout is needed to meet contrast, touch-target or platform requirements, do so; do not silently change what a semantic token *means*. Record lasting deviations via `PLATFORM_EXCEPTIONS.md`.

## 5. Consistency ≠ feature parity

Feature parity is an explicit product decision. Examples:

- The web may lack a global hotkey or tray integration.
- Linux desktop support depends on display server, compositor, toolkit and distribution.
- Mobile capture sessions can be interrupted/background-restricted.
- A platform may only support selected local inference backends.

Mark features **Supported / Partially supported / Unsupported / Not implemented / Not verified**, with a dated evidence note. Do not design fake or dead controls to imply parity.

## 6. Anti-patterns

- Duplicating exact token values in every platform guide or specialist.
- Replicating macOS window buttons in Windows/Linux/Web.
- Using purple for recording on one platform and green on another without semantic reason.
- Suppressing focus rings to match screenshots.
- Assuming tested on one desktop equals tested everywhere.
- Requiring a tray/menu-bar icon for essential actions.
- Treating any visual difference as a defect when the platform has a better native pattern.
