# RimV Design System

**Status:** RimV proposed baseline; verify actual app implementation before migrating existing UI.  
**Canonical location:** `.aiassistance/design/RIMV_DESIGN_SYSTEM.md`  
**Consumers:** UI/UX Designer, Frontend, Swift, Windows and Linux specialists; `ui-design`, `design-systems`, `usability-heuristics`, `responsive-adaptive-ui`, and existing `accessibility` skills.

This is the single source of truth for RimV's shared visual identity and interaction semantics. Platform-specific documents under `platforms/` specify native adaptations. Design tokens live in `tokens/rimv.tokens.json`. Do not copy token tables into agent definitions.

## 1. Source and precedence

RimV high-focus assistant: calm neutrals, systematic spacing, semantic green for live capture, restrained purple for AI, strong transcript readability, and a neutral dark-mode action style.

Priority for UI changes:

1. Accessibility, clear recording/privacy state, and functional requirements.
2. Existing approved RimV product decisions, if documented.
3. This shared design system and semantic tokens.
4. Target-platform interaction conventions.
5. Existing local component patterns where they don't conflict with 1–4.

For native controls, preserve the platform's behavior; adapt token mapping rather than imitating another operating system. Do not redesign existing screens globally unless requested.

## 2. Design principles

- **Clarity over decoration:** prioritise user task and state over effects.
- **Calm, high-focus UI:** neutral surfaces dominate; accents convey meaning.
- **Immediate feedback:** every initiated operation reveals what happened.
- **Recognition before recall:** relevant state and controls remain visible.
- **Responsive/adaptive:** reflow according to available space, not a fixed screenshot.
- **Native-feeling:** keyboard, menus, windows, sheets and notifications follow the host platform.
- **Privacy transparency:** never conceal whether audio capture is active.
- **A11y by construction:** keyboard, touch, contrast, zoom/scaling and screen-reader use are design inputs, not late fixes.

## 3. Semantic colors

The table below written design guide. Exact channel values must be implemented as semantic tokens, not scattered literals.

| Token | Light | Dark | Role |
|---|---|---|---|
| `background` | `#EEF2FA` | `#0B0F19` | App shell |
| `surface` | `#FFFFFF` | `#111827` | Main panel/card |
| `surface-secondary` | `#F6F7FB` | `#172033` | Grouped controls/cards |
| `surface-tertiary` | `#ECE9FF` | `#1F2937` | Selected/raised neutral surfaces |
| `border` | `#E3E6EF` | `#263244` | Thin boundaries |
| `border-strong` | `#D7DDED` | `#374151` | Emphasized outline |
| `text-primary` | `#171827` | `#F9FAFB` | Essential text |
| `text-secondary` | `#5E6475` | `#CBD5E1` | Descriptions/metadata |
| `text-muted` | `#9AA3B5` | `#94A3B8` | Decorative or disabled metadata only when contrast permits |
| `icon-primary` | `#171827` | `#F9FAFB` | Primary iconography |
| `icon-secondary` | `#7A8194` | `#CBD5E1` | Secondary iconography |
| `brand` | `#8B6FF6` | `#A78BFA` | RimV purple brand accent |
| `brand-hover` | `#7C5EF0` | `#C4B5FD` | Visual hover accent, contrast to be tested |
| `brand-pressed` | `#6D4DE6` | `#8B5CF6` | Press state |
| `action-solid` | `#6D4DE6` | `#2F3747` | High-emphasis button fill with readable light text |
| `action-solid-hover` | `#6241D0` | `#3A4558` | Hover for solid actions |
| `action-solid-pressed` | `#5638BB` | `#242C3A` | Pressed action |
| `on-action` | `#FFFFFF` | `#F9FAFB` | Solid-action label |
| `ai` | `#7C3AED` | `#A78BFA` | AI-generated information/icon accent |
| `live` | `#16A34A` | `#22C55E` | Live recording/capture |
| `warning` | `#B45309` | `#FBBF24` | Warnings |
| `error` | `#DC2626` | `#F87171` | Errors/destructive operations |
| `focus` | `#6D4DE6` | `#C4B5FD` | Visible keyboard focus |

**Important contrast decision:** light brand `#8B6FF6` with white text has approximately 3.7:1 contrast and therefore must *not* be used as a small-text button fill under a 4.5:1 target. The deeper `action-solid` `#6D4DE6` with white text is selected for readable filled actions. Light `text-muted` on white is too faint for ordinary meaningful text: use `text-secondary` for timestamps, descriptions and labels. Verify specific on-surface pairs before shipping; do not assume all combinations of approved tokens are accessible.

### Semantic message surfaces

| State | Light BG / FG | Dark BG / FG |
|---|---|---|
| Info | `#DBEAFE` / `#1D4ED8` | `#172554` / `#93C5FD` |
| Success | `#D1FAE5` / `#065F46` | `#14532D` / `#86EFAC` |
| Warning | `#FEF3C7` / `#92400E` | `#78350F` / `#FCD34D` |
| Error | `#FEE2E2` / `#991B1B` | `#7F1D1D` / `#FCA5A5` |

Use text + icon + color for important states. A disabled control must remain recognizable; opacity alone is insufficient when it hides an important label.

## 4. Typography

Prefer installed system fonts. Do not redistribute Apple's proprietary font files.

| Platform | UI | Display | Monospace |
|---|---|---|---|
| macOS/iOS | System (SF Pro family when available) | System display typography | System monospaced |
| Windows | Segoe UI / system | Segoe UI Variable where available | Cascadia Mono/system |
| Linux | Toolkit/system UI; Inter where available | System UI | System monospace |
| Web | `Inter, system-ui, sans-serif` | Same | `ui-monospace, monospace` |

| Text style | Desktop size/line | Small-screen size/line | Weight |
|---|---|---|---|
| Display | 36/42 | 34/40 | 600 |
| H1 | 30/36 | 28/34 | 600 |
| H2 | 24/30 | 22/28 | 600 |
| H3 | 18/24 | 18/24 | 600 |
| Transcript / body large | 16/24 | 16/22 | 400 |
| Body | 14/22 | 14/20 | 400 |
| Button | 14/18 | 16/20 | 600 / native equivalent |
| Footnote | 12/16 | 12/16 | 400–500 |
| Caption | 11/14 | 11/14 | 500 |

Primary transcript body should use **at least the 16-size body-large token**, allow user scaling and comfortable line spacing. Core explanatory text should not be reduced to captions. The fixed sizes above are starting values, not caps on Dynamic Type, browser zoom or desktop text scaling.

## 5. Spacing, geometry and density

- Spacing scale: `4, 8, 12, 16, 20, 24, 32, 40, 48`.
- Radius: `xs 6`, `sm 8`, `md 12`, `lg 16`, `xl 20`, `full 999`.
- Panel padding: `24`; card padding: `16` or `20`.
- Section gap: `24` or `32`; card-stack gap: `12` or `16`; toolbar gap: `8` or `12`.
- Icon-label gap: `8`; icon sizes: `16, 18, 20, 24`.
- Standard desktop button/input height: `40`; touch-oriented button `48`, input `44–48`.
- Status chips: height `26`, horizontal padding `10–12`, fully rounded.
- Controls aligned in the same row should have consistent heights and visual baseline.

Hit target is not identical to visible control size. Keep mobile touch targets approximately `44×44` or larger where practical; web WCAG 2.2 AA's target-size criterion is a distinct minimum with exceptions. See the existing `accessibility` skill.

## 6. Layout hierarchy

Default desktop workspace, when screen size and actual RimV features justify it:

- Navigation/sidebar: approximately `240–280` wide.
- Primary workspace: flexible; aim for `>=720` when a three-column layout is used.
- Context/AI panel: approximately `320–380` wide.
- Major panel spacing: `20–24`.

Do not force three columns onto smaller windows. Collapse the contextual panel first, then sidebar; primary transcript and capture controls must remain usable. Mobile uses a single main column with secondary views in navigation destinations or sheets. Never shrink all text just to preserve a desktop layout.

Default visual priority: **recording status → transcript/current work → AI results → primary controls → secondary settings → diagnostics**. Controls needed to stop recording remain discoverable even when the transcript occupies most of the page.

## 7. Components

### Buttons

- Primary: one principal action per local action group; solid fill uses accessible `action-solid` tokens.
- Secondary: `surface-secondary` + `border` + `text-primary`.
- Ghost: transparent or subtle surface, no unnecessary glow.
- Destructive: red plus explicit label (for example `Delete recording`).
- Icon-only controls require accessible labels and discoverable tooltips on pointer devices.
- Press feedback: approximately 100–140 ms; hover: 120–160 ms; avoid perpetual idle animations.
- Always show busy/disabled state when an action can't be repeated safely.

### Inputs, selects and dialogs

- Persistent labels for settings; placeholders give examples rather than substitute labels.
- Visible focus ring, error helper text and keyboard navigation.
- Prefer native selects, switches, sheets and dialogs when appropriate to the platform.
- Preserve user-entered values on recoverable failures.
- Don't trigger destructive side effects from a selection change without clear affordance.

### Cards, panels and elevation

- Card radius `16`, card padding `16–20`, panel padding `24`.
- Light card shadow, if needed: `0 1px 2px rgba(15,23,42,.06), 0 6px 18px rgba(15,23,42,.06)`.
- Dark cards generally use borders/surface differences, not large shadows.
- Distinguish live transcript, finalized transcript and AI-generated summary visually and in text.

### Iconography

Use one consistent family per platform: SF Symbols on Apple, Fluent icons on Windows, toolkit-native icons on Linux, and one selected SVG library on Web. Match icon weight within a toolbar; reserve fill changes for meaningful selection/active state. Do not copy SF Symbol artwork into non-Apple applications.

## 8. Audio, transcription and AI states

Use `patterns/audio-transcription.md` and `patterns/model-management.md` for full state flows.

**Recording states:** `idle → requesting-permission → preparing → live ↔ paused → stopping → finalizing → completed`; possible exits include `permission-denied`, `device-unavailable`, `error` and `cancelled`.

- **Live:** persistent green dot + `Recording` or `Listening` label and a visible Stop control. Animation supplements but never replaces text.
- **Paused:** clearly distinct from live; stop remains reachable; do not imply microphone is live when it is paused.
- **Partial transcript:** visually marked as provisional, not silently treated as final.
- **Final transcript:** preserve copy/export and speaker/time semantics where available.
- **AI output:** purple semantic marker and a clear label such as `AI summary`; never silently merge generated text with verbatim transcript.
- **Model downloads:** show current step, progress if measurable, cancellation where safe, retry after recoverable failure.
- **Microphone permissions:** explain purpose, respect denial, provide platform-specific recovery without repeatedly prompting.

## 9. Motion and feedback

| Motion | Duration |
|---|---:|
| Press | 100–140 ms |
| Hover | 120–160 ms |
| Component change | 160–220 ms |
| Tab change | 120–150 ms |
| Modal/sheet | 240–300 ms |
| Restrained live pulse | 1.5–2 s |

Waveforms may animate while audio is live but must stop when capture ends. Reduced-motion mode removes nonessential pulses, translation, shimmer and parallax; user state remains equally clear.

## 10. Accessibility and inclusion

Web target: WCAG 2.2 AA; use equivalent native-platform accessibility patterns elsewhere. This is a development goal, **not a certification claim**.

- Normal text contrast target: at least 4.5:1; large text: 3:1; meaningful graphical objects/focus states: at least 3:1 where applicable.
- Use keyboard-operable controls and visible focus; don't steal focus during streaming transcription.
- Ensure status/error/progress descriptions are available to assistive tech without announcing every fast-changing token.
- Respect browser zoom, Dynamic Type, desktop text scaling, reduced motion and high-contrast/forced-color settings.
- Make controls operable without drag-only gestures; verify pointer targets and spacing.
- Support long translated labels, RTL layouts when localization is introduced and transcript text selection.
- Do not require color, sound or animation alone to understand recording state.

For exact checks use the existing `.aiassistance/skills/accessibility/SKILL.md`; this design document does not duplicate its test workflow.

## 11. Usability heuristics and content

Review important workflows with Nielsen's ten heuristics via `.aiassistance/skills/usability-heuristics/SKILL.md`. In RimV, give particular scrutiny to permission denial, recording stop/cancel, model selection, partial results and recovery.

Microcopy:

- Prefer specific verbs: `Start listening`, `Pause`, `Stop`, `Download model`, `Retry`.
- Show what happened and the next action: `Microphone access is off. Open system settings to enable it.`
- Avoid internal engine terms for ordinary users; offer details in expandable diagnostics.
- Destructive actions must identify exactly what is deleted; use confirmation when recovery is impossible.
- Don't imply transcription or AI output is definitive when partial, delayed or uncertain.

## 12. Theme and platform mapping

Every platform maps semantic tokens into its native system; identity is shared but controls needn't be pixel-identical. See `platforms/apple-macos.md`, `apple-ios.md`, `windows.md`, `linux.md`, `web.md`.

Prefer native system theme when no explicit user choice exists. Persist an explicit user override if settings provide one. Test light/dark, high contrast and system text scaling. Avoid declaring Windows/Linux interfaces “native” unless their implementation actually uses native platform APIs.

## 13. Design QA release gate

- [ ] Capture state, Stop action and privacy behavior are obvious.
- [ ] Primary and secondary actions are distinguishable.
- [ ] Main text and relevant token combinations pass contrast tests.
- [ ] Light/dark theme and focus states are verified.
- [ ] Keyboard/touch/navigation work for the target platform.
- [ ] Responsive layout preserves transcript readability.
- [ ] Permission, loading, empty, partial/final, error and retry states are implemented where relevant.
- [ ] Screen-reader status announcements are deliberate and not excessively noisy.
- [ ] Reduced motion and text scaling are supported.
- [ ] UI follows applicable platform conventions.
- [ ] AI text is clearly distinct from source transcript.
- [ ] Relevant usability issues have an actionable disposition.

## 14. Documentation and change policy

If a design decision changes: edit this document first, update `tokens/rimv.tokens.json`, then adjust affected platform guidelines and components. Document deviations with the reason and platform; never introduce a competing copy of the palette inside agent/skill files. Use `.aiassistance/workflows/ui-implementation.md` for a full design-to-code task.

### External references

- Nielsen Norman Group: https://www.nngroup.com/articles/ten-usability-heuristics/
- W3C WCAG 2.2: https://www.w3.org/TR/WCAG22/
- Apple Human Interface Guidelines: https://developer.apple.com/design/human-interface-guidelines/
- Microsoft Fluent 2: https://fluent2.microsoft.design/
- GNOME HIG: https://developer.gnome.org/hig/
