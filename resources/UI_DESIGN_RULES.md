# rimv Web UI Design Rules

> Purpose: This document is the single source of truth for Codex when implementing the rimv web interface.
>
> Goal: building a lightweight browser client.

---

## 1. Product UI Goal

- Calm, focused, professional
- Dark-first visual identity, with complete light-mode support
- Transcript-first layout
- Strong hierarchy
- Rounded panels and cards
- Minimal visual noise
- Purple for AI/intelligence
- Green for live/listening/recording state
- Red only for destructive or stop actions
- Blue for system/source informational states
- Fast, responsive, lightweight

The UI is a thin client.

Heavy processing remains in the rimv Rust engine:

```text
Browser UI
TypeScript + Vite + Tailwind CSS
        │
        │ WebSocket / API
        ▼
rimv Rust Engine
        │
        ├─ Capture
        ├─ Whisper
        ├─ Recording
        └─ Transcript events
```

Do not move Whisper or heavy audio processing into the browser in this phase.

---

## 2. Frontend Stack

Use:

```text
TypeScript
Vite
Tailwind CSS
Native WebSocket
HTML
CSS variables for design tokens
```

Do NOT use:

```text
React unless truly necessary later
Electron
Angular
Tailwind CDN
large UI kits
heavy animation libraries
runtime CSS-in-JS
```

Tailwind must be compiled at build time.

The production browser must receive only static CSS generated for the classes actually used.

---

## 3. Design Principles

1. Clarity before decoration.
2. Transcript content is more important than controls.
3. The interface should remain visually calm during long sessions.
4. Use color sparingly.
5. State must be understandable at a glance.
6. Keep interaction patterns consistent.
7. Dark and light modes must have equivalent hierarchy.
8. Avoid visual clutter.
9. Prefer native-feeling controls.
10. Keep the browser client lightweight.

---

# 4. Layout System

Use an 8 px spacing grid.

Recommended desktop layout:

```text
┌──────────────────────────────────────────────────────────────┐
│ rimv                                                │
├──────────────┬─────────────────────────────┬─────────────────┤
│              │                             │                 │
│ Left Sidebar │ Main Session Workspace      │ Right Info      │
│              │                             │ Panel           │
│ ~260 px      │ flexible                    │ ~320-350 px     │
│              │                             │                 │
└──────────────┴─────────────────────────────┴─────────────────┘
```

### Widths

- Sidebar: `260px`
- Right panel: `320–350px`
- Main content: flexible
- Minimum comfortable desktop width: approximately `1180px`

### Spacing

- Page padding: `24px`
- Major section gap: `24–32px`
- Card gap: `12–16px`
- Card padding: `16–20px`
- Main panel padding: `24px`
- Compact element gap: `8px`

### Radius

- Main cards: `16px`
- Smaller cards: `12px`
- Buttons: `10–12px`
- Status chips: pill / `9999px`
- Inputs: `10–12px`

---

# 5. Color Philosophy

Approximate visual balance:

```text
75% neutral surfaces/text
20% borders/elevation
5–10% accent colors
```

Do not flood the interface with purple, green, or blue.

Semantic meaning:

| Color | Meaning |
|---|---|
| Purple | AI, intelligence, selected navigation |
| Green | Live, listening, recording active, success |
| Blue | System audio, informational state |
| Red | Stop, destructive actions, critical errors |
| Amber | Warning |
| Neutral | General structure/content |

Never use semantic colors randomly for decoration.

---

# 6. Dark Theme

Dark mode is the primary rimv identity.

```css
--bg: #090B12;
--surface: #11131D;
--surface-secondary: #171A27;
--surface-tertiary: #201A35;

--border: #292D3A;
--border-strong: #3A3F52;

--text-primary: #F9FAFB;
--text-secondary: #CBD5E1;
--text-muted: #94A3B8;

--ai-primary: #A78BFA;
--ai-secondary: #C4B5FD;

--live: #22C55E;
--system: #3B82F6;
--warning: #FBBF24;
--danger: #F87171;
```

### Dark surfaces

Hierarchy:

```text
App background       #090B12
Main cards           #11131D
Secondary cards      #171A27
Selected/AI surfaces #201A35
```

Avoid pure black `#000000`.

---

# 7. Light Theme

```css
--bg: #F8F8FB;
--surface: #FFFFFF;
--surface-secondary: #F4F2FA;
--surface-tertiary: #F1ECFF;

--border: #E6E3EF;
--border-strong: #D8D2E8;

--text-primary: #111827;
--text-secondary: #5E6475;
--text-muted: #9AA3B5;

--ai-primary: #8B5CF6;
--ai-secondary: #A78BFA;

--live: #16A34A;
--system: #2563EB;
--warning: #F59E0B;
--danger: #EF4444;
```

Do not simply invert dark colors.

Preserve the same hierarchy and semantic meaning.

---

# 8. Theme Selection

Support:

```text
System
Light
Dark
```

Default:

```text
System
```

Use CSS variables as the canonical token layer.

Example architecture:

```text
Design Tokens
    │
    ├── Light values
    └── Dark values
          │
          ▼
      Tailwind
          │
          ▼
      Components
```

Components must use semantic design tokens.

Do NOT scatter raw hex colors across components.

Bad:

```text
bg-purple-500
text-gray-300
```

Preferred conceptually:

```text
bg-surface
text-primary
text-muted
bg-ai
text-live
border-default
```

---

# 9. Typography

Primary stack:

```css
font-family:
  -apple-system,
  BlinkMacSystemFont,
  "SF Pro Text",
  "Inter",
  "Segoe UI",
  sans-serif;
```

Monospace:

```css
"SF Mono",
"JetBrains Mono",
monospace;
```

### Type scale

| Role | Size | Line Height | Weight |
|---|---:|---:|---:|
| H1 | 30px | 36px | 600 |
| H2 | 24px | 30px | 600 |
| H3 | 18px | 24px | 600 |
| Transcript | 16px | 24px | 400–500 |
| Body | 14px | 22px | 400 |
| Metadata | 12px | 16px | 400–500 |
| Small label | 11–12px | 14–16px | 500 |

Transcript text must remain easy to read for long sessions.

Do not reduce transcript text to tiny UI-label size.

---

# 10. Component Rules

## Buttons

Primary action:
- Purple
- Clear text
- Medium emphasis

Live/recording action:
- Green only when action represents active/live state

Stop/destructive:
- Red
- Must not be visually confused with primary action

Secondary:
- Neutral surface with border

Recommended height:

```text
40px
```

Compact icon button:

```text
36–40px
```

---

## Toggle Controls

Use for:

```text
Microphone
System Audio
Transcription
```

States:

```text
Off → neutral
On → blue or semantic state
Unavailable → muted/disabled
Error → red indicator
```

Do not use color as the only state indicator.

Text/icon/state must remain understandable.

---

## Status Chips

Examples:

```text
● Live
● Recording
● Transcribing
AI Ready
Model Loaded
```

Use compact pill shapes.

Green:
- live
- recording
- ready success

Purple:
- AI state
- transcription capability where appropriate

Red:
- error only

---

## Cards

All cards should have:

```text
surface background
1px border
16px radius
16–20px padding
```

Avoid strong shadows in dark mode.

Use borders and subtle tonal elevation instead.

---

# 11. Main Session Screen

The main experience should prioritize:

1. Session title/status
2. Capture controls
3. Live waveform/activity
4. Live transcript
5. Secondary system/model information

Suggested structure:

```text
Session header
────────────────────────────
Recording / Transcribing status

Live capture card
────────────────────────────
Waveform
Mic toggle
System Audio toggle
Transcription toggle
Pause / Stop

Transcript
────────────────────────────
Live Transcript
Speakers & Sources
Session Info

Right panel
────────────────────────────
Transcription Status
System Information
Recent Activity
```

---

# 12. Transcript UI

Transcript is the primary content.

Each final segment should display:

```text
Timestamp
Source
Text
```

Example:

```text
23:15:03   Microphone
I think we should release this tomorrow.

23:15:12   System
Yes, that works for me.
```

Source badges:

```text
Microphone → purple
System     → blue
```

Do not imply these are different human speakers.

Source ≠ speaker diarization.

### Partial transcript

Partial text should appear visually softer:

- lower opacity
- subtle italic optional
- never look identical to finalized transcript

When finalized, replace/update it cleanly.

Do not duplicate partial + final versions.

---

# 13. Waveform / Listening Indicator

The waveform is functional feedback, not decoration.

Use:

```text
green while actively listening
neutral when idle
```

Keep animation inexpensive.

Prefer CSS transforms / requestAnimationFrame.

Do not run expensive canvas effects unnecessarily.

Respect:

```text
prefers-reduced-motion
```

---

# 14. Sidebar

Structure:

```text
Brand
Search

Sessions
  active session
  previous sessions

Navigation
  Home
  Sessions
  Models
  Settings

User/Profile
```

Selected navigation:

- subtle purple surface
- purple icon/text accent
- no excessively bright background

Active session:
- clearly selected
- live/active indicator
- may use subtle purple border/background

---

# 15. Model Settings UI

Whisper models page should use the same visual system.

Example:

```text
Models

Whisper
Speech transcription model

Recommended for this device: Small

Small
Higher accuracy
~466 MB
[ Download ]

Base
Balanced
[ Download ]

Tiny
Fastest / low memory
[ Download ]
```

During download:

```text
Downloading Small
██████████████░░ 73%

341 MB / 466 MB
[ Cancel ]
```

Installed:

```text
Small
✓ Installed
Current model

[ Use ] [ Remove ]
```

Do not place download/business logic in UI.

The UI consumes backend state/events.

---

# 16. Responsive Rules

Desktop is primary.

For narrower screens:

### ≥ 1200 px
Three columns:

```text
sidebar | content | info
```

### 768–1199 px
Hide/collapse right panel.

```text
sidebar | content
```

### < 768 px
Single-column layout.

```text
top navigation
content
bottom/overlay controls
```

On mobile/browser controller:

Prioritize:

```text
status
start/stop
mic
system
transcription
live transcript
```

Do not attempt to reproduce every desktop panel on a phone.

---

# 17. Accessibility

Required:

- Keyboard navigation
- Visible focus states
- Semantic HTML
- Proper labels
- ARIA only when needed
- Sufficient contrast
- Do not rely only on color
- Support `prefers-reduced-motion`
- Buttons must have accessible names
- Toggle state must be announced
- Transcript text must be selectable

Minimum touch target:

```text
44 × 44 px
```

where appropriate on mobile.

---

# 18. Motion

Motion should feel calm.

Recommended:

```text
120–180ms
ease-out
```

Use for:

- hover
- menu opening
- status transitions
- button feedback

Avoid:

- bouncing
- large spring effects
- constant glowing
- unnecessary animated gradients

Live waveform may animate continuously because it conveys state.

---

# 19. Tailwind Rules

Use Tailwind for:

```text
layout
grid
flex
spacing
responsive behavior
hover/focus states
typography utilities
basic component structure
```

Use CSS variables for rimv semantic tokens.

Do NOT use Tailwind CDN.

Do NOT hardcode arbitrary colors repeatedly:

```text
bg-[#11131D]
text-[#A78BFA]
```

except temporarily during prototyping.

Create semantic theme utilities/tokens instead.

---

# 20. CSS Rules

Native CSS is still allowed for:

- design tokens
- complex component states
- waveform visualization
- custom animations
- scrollbar styling
- platform-specific refinements

Do not force every rule into Tailwind when plain CSS is clearer.

Recommended approach:

```text
Tailwind = layout/composition
CSS variables = design system
Small native CSS = specialized behavior
```

---

# 21. Performance Rules

The browser is a thin UI.

Targets:

- very small bundle
- no heavy framework initially
- no unnecessary rerenders
- bounded transcript DOM/history
- virtualize only if sessions become very large
- avoid polling when WebSocket events are available
- use event-driven UI
- lazy load secondary settings/pages
- avoid expensive blur effects
- avoid huge SVG libraries

Icons:
- use a lightweight icon package or local SVGs
- tree-shake icons
- never load an entire icon set at runtime

---

# 22. WebSocket UI Architecture

UI state must come from rimv events.

Concept:

```text
User interaction
      │
      ▼
EngineCommand
      │
      ▼
rimv
      │
      ▼
EngineEvent / Snapshot
      │
      ▼
UI state
```

Example commands:

```text
StartCapture
StopCapture
SetMicrophoneEnabled
SetSystemAudioEnabled
SetTranscriptionEnabled
```

Example events:

```text
Snapshot
TranscriptPartial
TranscriptFinal
Error
ModelDownloadProgress
```

Do not invent independent frontend state that can contradict the engine.

rimv is the source of truth.

---

# 23. UI State Rules

Every control must support:

```text
normal
hover
pressed
focus
disabled
loading
error where relevant
```

Example transcription button:

```text
Unavailable
Loading model
Ready
Transcribing
Error
```

Do not collapse all backend states into only On/Off.

---

# 24. Error Presentation

Do not use modal alerts for every small error.

Use:

- inline error for local component issue
- toast for temporary action feedback
- prominent banner only for session-critical issue

Audio recording should visually remain active if transcription alone fails.

Example:

```text
Recording    ● Active
Transcription ⚠ Model unavailable
```

Do not imply the whole session failed.

---

# 25. Naming Rules

Use user-facing language:

```text
Microphone
System audio
Transcription
Live transcript
Models
Settings
Recording
Listening
```

Avoid exposing implementation terms:

```text
CPAL
ScreenCaptureKit
whisper-rs
bounded queue
engine-runtime
AudioFrame
```

Those belong in diagnostics/developer areas only.

---

# 26. Visual Identity

The identity should feel like:

```text
rimv voice/transcription purpose
```

Purple:
- intelligence/family identity

Green:
- active listening/live capture

Blue:
- system source

---

# 27. Forbidden Patterns

Do NOT:

- use random colors
- use pure black backgrounds everywhere
- use giant gradients
- add glassmorphism everywhere
- overuse blur
- overload cards with shadows
- use neon colors without semantic meaning
- create inconsistent radii
- make transcript text too small
- hide critical recording state
- duplicate backend state locally
- add large JS UI libraries without need
- add Electron for the browser client
- use Tailwind Play CDN in production
- make dark/light themes separate component implementations

---

# 28. Codex Implementation Rule

Before implementing a new UI component, Codex should ask:

```text
1. What semantic role does this component have?
2. Which existing design token applies?
3. Does an existing component already solve it?
4. Is the state coming from rimv or duplicated locally?
5. Does it work in both light and dark mode?
6. Is keyboard accessibility preserved?
7. Does it remain lightweight?
```

If a component requires a new color, spacing value, or interaction pattern, prefer extending the design system rather than hardcoding it locally.

---

# 29. Suggested Project Structure

```text
web/
├── src/
│   ├── components/
│   │   ├── Button/
│   │   ├── Card/
│   │   ├── Toggle/
│   │   ├── StatusChip/
│   │   ├── Transcript/
│   │   └── Waveform/
│   │
│   ├── features/
│   │   ├── sessions/
│   │   ├── recording/
│   │   ├── transcription/
│   │   └── models/
│   │
│   ├── engine/
│   │   ├── protocol.ts
│   │   ├── websocket.ts
│   │   └── state.ts
│   │
│   ├── styles/
│   │   ├── tokens.css
│   │   ├── globals.css
│   │   └── animations.css
│   │
│   └── main.ts
│
├── index.html
├── tailwind.config.*
├── vite.config.*
└── package.json
```

Keep component and feature boundaries simple.

Do not over-engineer.

---

# 30. Final Visual Target

The finished application should visually communicate:

```text
Dark professional workspace
↓
clear live recording state
↓
simple capture controls
↓
highly readable realtime transcript
↓
secondary model/system information
```

The visual hierarchy should be:

```text
1. Session / recording state
2. Transcript
3. Capture controls
4. Navigation
5. Diagnostics/model/system information
```
