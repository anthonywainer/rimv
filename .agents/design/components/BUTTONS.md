# Buttons and Action Controls

**Tokens:** `../tokens/rimv.tokens.json`  
**Shared rules:** `../RIMV_DESIGN_SYSTEM.md`

## Button inventory

| Type | Purpose | Appearance | Behavior |
|---|---|---|---|
| Primary | Main actionable step in the current group | Accessible `action-solid` fill + `on-action` label | Only one per group when practical |
| Secondary | Alternative but visible action | `surface-secondary`, 1px `border`, `text-primary` | Same height as primary in a row |
| Ghost | Utility or low-priority action | Transparent/subtle surface | Strong hover/focus feedback |
| Destructive | Irreversible action | Error treatment **plus explicit text** | Ask confirmation when irreversible and impactful |
| Icon | Compact toolbar utility | 40×40 visible (desktop); adequate touch hit area | Accessible name and pointer tooltip |
| Round audio | Primary recording control when appropriate | 56×56 desktop option; state-dependent | Start/Stop label is still visible elsewhere |

## Dimensions

- Desktop buttons: 40px high. Touch-oriented controls: 48px high.
- Icon-to-text gap: 8px. Horizontal padding: 12–16px.
- Square icon buttons: 40×40px; make the hit target larger when required.
- Default radius: `md` (12px); pill/full radius for segmented choices and status controls.
- Align buttons on common baseline and matching height within one toolbar.

## Primary green-teal contrast

RimV uses green-teal for brand and primary actions. The light-mode primary action uses `#138D84` with a large bold white label/icon (approximately 4.1:1; above the 3:1 large-text threshold). Keep smaller text on the darker `#0B6E69`; dark-mode primary action also remains `#0B6E69`. Purple is reserved for explicitly AI-generated information; red remains reserved for error, destructive and Stop treatments. Accent-outline buttons are an optional alternate pattern, not a mandatory replacement for primary actions.

## State contract

- `default` — action ready;
- `hover` — only on hover-capable devices; 120–160ms transition;
- `focus-visible` — strongly visible outline/ring, independently of hover;
- `pressed` — 100–140ms; approximately .98 scale only where platform-native;
- `busy` — spinner/progress + prevent accidental repeated side effects;
- `disabled` — readable and unmistakably unavailable; preserve explanation where necessary;
- `destructive-confirmation` — clearly names affected recording/model/data.

Do not use hover as the only way to discover an essential control. Always preserve a usable Stop action during active capture.

## Component acceptance criteria

- Keyboard activation, focus order and accessible names work.
- In one toolbar, action groups are visually coherent.
- Small labels on any filled color meet applicable contrast requirements.
- Long translated labels can wrap or grow without truncating the primary action.
- Reduced motion preserves state communication.
