# Proposed Part 4A Design Token — Contrast Audit

Computed from `.aiassistance/design/tokens/rimv.tokens.json` **as packaged in Part 4A**, not from the current rendered application.
Ratios use the W3C relative luminance contrast formula. Values are rounded to two decimals. A pass/fail statement is conditional on the actual semantic *use* of a token.

## Pair checks

### Light theme

| Foreground | Background | Ratio | Comparison | Note |
|---|---|---:|---|---|
| `text-primary` #171827 | `surface` #FFFFFF | 17.55:1 | Meets numeric target (4.5:1) | ordinary text |
| `text-secondary` #5E6475 | `surface` #FFFFFF | 5.91:1 | Meets numeric target (4.5:1) | ordinary text |
| `text-muted` #9AA3B5 | `surface` #FFFFFF | 2.54:1 | Below numeric target (4.5:1) | if used as small essential text |
| `brand` #8B6FF6 | `surface` #FFFFFF | 3.68:1 | Below numeric target (4.5:1) | if used as small text |
| `live` #16A34A | `surface` #FFFFFF | 3.30:1 | Below numeric target (4.5:1) | if used as small text |
| `ai` #7C3AED | `surface` #FFFFFF | 5.70:1 | Meets numeric target (4.5:1) | if used as small text |
| `warning` #B45309 | `surface` #FFFFFF | 5.02:1 | Meets numeric target (4.5:1) | if used as small text |
| `error` #DC2626 | `surface` #FFFFFF | 4.83:1 | Meets numeric target (4.5:1) | if used as small text |
| `on-action` #FFFFFF | `action-solid` #6D4DE6 | 5.45:1 | Meets numeric target (4.5:1) | button text |
| `focus` #6D4DE6 | `surface` #FFFFFF | 5.45:1 | Meets numeric target (3.0:1) | meaningful focus indicator |
| `border` #E3E6EF | `surface` #FFFFFF | 1.25:1 | Below numeric target (3.0:1) | if sole essential control boundary |
| `border-strong` #D7DDED | `surface` #FFFFFF | 1.36:1 | Below numeric target (3.0:1) | if sole essential control boundary |

### Dark theme

| Foreground | Background | Ratio | Comparison | Note |
|---|---|---:|---|---|
| `text-primary` #F9FAFB | `surface` #111827 | 16.98:1 | Meets numeric target (4.5:1) | ordinary text |
| `text-secondary` #CBD5E1 | `surface` #111827 | 11.95:1 | Meets numeric target (4.5:1) | ordinary text |
| `text-muted` #94A3B8 | `surface` #111827 | 6.92:1 | Meets numeric target (4.5:1) | if used as small essential text |
| `brand` #A78BFA | `surface` #111827 | 6.52:1 | Meets numeric target (4.5:1) | if used as small text |
| `live` #22C55E | `surface` #111827 | 7.79:1 | Meets numeric target (4.5:1) | if used as small text |
| `ai` #A78BFA | `surface` #111827 | 6.52:1 | Meets numeric target (4.5:1) | if used as small text |
| `warning` #FBBF24 | `surface` #111827 | 10.63:1 | Meets numeric target (4.5:1) | if used as small text |
| `error` #F87171 | `surface` #111827 | 6.41:1 | Meets numeric target (4.5:1) | if used as small text |
| `on-action` #F9FAFB | `action-solid` #2F3747 | 11.43:1 | Meets numeric target (4.5:1) | button text |
| `focus` #C4B5FD | `surface` #111827 | 9.61:1 | Meets numeric target (3.0:1) | meaningful focus indicator |
| `border` #263244 | `surface` #111827 | 1.37:1 | Below numeric target (3.0:1) | if sole essential control boundary |
| `border-strong` #374151 | `surface` #111827 | 1.72:1 | Below numeric target (3.0:1) | if sole essential control boundary |

## Candidate accessible-use mappings (not applied)

These are **examples for product review**, not automatic modifications to the canonical design system. Test against the actual rendered surface, including borders, opacity, gradients and platform high-contrast mode.

| Intended use | Candidate | Verified background ratios |
|---|---|---|
| light essential muted text | `#626A7D` | `#FFFFFF` 5.42:1, `#F6F7FB` 5.06:1, `#EEF2FA` 4.83:1 |
| light small-text brand/accent | `#6D4DE6` | `#FFFFFF` 5.45:1, `#F6F7FB` 5.09:1, `#EEF2FA` 4.86:1 |
| light small-text live green | `#166534` | `#FFFFFF` 7.13:1, `#F6F7FB` 6.66:1, `#EEF2FA` 6.36:1 |
| light essential control outline | `#80899A` | `#FFFFFF` 3.52:1, `#F6F7FB` 3.29:1, `#EEF2FA` 3.14:1 |
| dark essential control outline | `#64748B` | `#111827` 3.73:1, `#172033` 3.42:1, `#0B0F19` 4.02:1 |

## Interpretation

- Light `text-muted` is **too low contrast for essential small text on white**. It can remain decorative or be replaced at the point of use by a verified accessible text role.
- Light `brand` and `live` are not suitable as ordinary small text on a white surface at the 4.5:1 threshold. They may still work for meaningful non-text graphics or large text if the corresponding criterion and background are satisfied.
- Ordinary `border` and `border-strong` tokens in both themes are intentionally subtle. If an outline is the **only visual cue** that an input/control exists or is selected, use a separate stronger accessible-outline token or redundant high-contrast state treatment.
- Text contrast, non-text contrast and focus appearance are separate requirements. A color passing one numeric check does not mean all component states are accessible.
- WCAG AA includes exceptions: incidental text, inactive controls and some special content are treated differently. Do not mark every decorative separator as a WCAG failure.
- Do not declare the app WCAG compliant from this token audit; actual rendered component pairs, overlays, motion and real interaction still need testing.

## Re-run locally

```bash
python3 .aiassistance/scripts/check-design-token-contrast.py
```

The script reads the installed Part 4A JSON. It emits conditional warnings but does not change files or return failure for expected proposal-level warnings.
