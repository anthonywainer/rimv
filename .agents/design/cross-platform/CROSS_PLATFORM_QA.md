# Cross-Platform UI Quality Assurance

Use this checklist for features spanning more than one platform. **Only mark a target Passed when it was actually tested.** Passing a desktop build or visual inspection in one environment is not universal evidence.

## 1. Establish scope

List each applicable target and current state:

- macOS (tested version / UI framework)
- iOS (tested device or simulator / OS version)
- Windows (tested OS / DPI / native framework)
- Linux (distribution, toolkit, display server, theme)
- Web (browser and tested viewport/zoom)

Status vocabulary: `Not implemented`, `Unsupported`, `Not verified`, `Blocked`, `Fail`, `Pass`. Report the reason for any blocked/not-verified target.

## 2. Core parity checklist

- [ ] Same feature has the same user-visible purpose where implemented.
- [ ] Recording indicator reflects confirmed engine state.
- [ ] Stop or safe recovery remains discoverable during capture.
- [ ] Partial vs final transcript is understandable.
- [ ] AI output is labeled separately from speech source.
- [ ] No fake download, export or permission controls are shown.
- [ ] Loading/progress values are truthful.
- [ ] Empty/unavailable/error states have next steps.
- [ ] Primary terminology agrees, allowing localization and OS-native terms.
- [ ] UI never overclaims local-only processing or retention guarantees.

## 3. Visual/token parity

- [ ] Implementations reference the approved canonical token or documented adapter.
- [ ] Light/dark hierarchy remains coherent where supported.
- [ ] Live, AI, warning, error and focus retain semantic meaning.
- [ ] Text/control contrast is checked in actual contexts.
- [ ] Accessible large-text/high-contrast overrides are supported where applicable.
- [ ] Icons are consistent within each platform, not necessarily identical across platforms.
- [ ] Responsiveness preserves primary content at available sizes.

## 4. Interaction parity

- [ ] Pointer/touch/keyboard route exists as appropriate.
- [ ] Focus order and return are logical.
- [ ] Screen-reader names announce control purpose and significant state changes.
- [ ] Modal/navigation behaviors follow target platform conventions.
- [ ] Device disconnect, permission denial and model-not-ready recovery are handled where relevant.
- [ ] No essential control depends on a tray, hover, gesture or global shortcut unavailable on the target.

## 5. Visual regression methodology

A screenshot comparison should flag unintentional drift **within the same platform/theme/size**, not demand pixel-level equivalence across different OSes. Keep per-platform baselines with recorded:

- OS/browser and version;
- window dimensions / browser zoom / scale factor;
- light/dark/high contrast;
- font scaling and language;
- data/state fixture;
- known acceptable native differences.

If screenshots are used, store images in an appropriate artifact/test folder, not duplicated inside `.aiassistance` instruction files. Review semantic/state assertions in addition to visuals.

## 6. Test matrix

| Scenario | macOS | iOS | Windows | Linux | Web |
|---|---|---|---|---|---|
| Idle/capture start/stop | — | — | — | — | — |
| Denied microphone access | — | — | — | — | — |
| Mid-session device loss / interruption | — | — | — | — | — |
| Model missing/download/ready | — | — | — | — | — |
| Long partial/final transcript | — | — | — | — | — |
| Error and retry | — | — | — | — | — |
| Light/dark and contrast | — | — | — | — | — |
| Keyboard/touch and assistive tech | — | — | — | — | — |
| Compact/large layout and font scale | — | — | — | — | — |

Replace `—` with the status vocabulary and a pointer to test evidence for each *implemented and applicable* target. Mark non-applicable scenarios explicitly.

## 7. Release gate

Flag as critical for release review:

- inaccurate recording state or no reliable stop mechanism;
- privacy or retention claims contradict implementation;
- inaccessible primary capture action;
- severe transcript loss/corruption or incorrect source/AI conflation;
- regression that breaks a previously supported target;
- known security-sensitive permission/file behavior regression.

Whether a platform/feature is ready for release is a human product decision; the reviewer reports evidence, blockers and limitations rather than inventing a pass for untested targets.
