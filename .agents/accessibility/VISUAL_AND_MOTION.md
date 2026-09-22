# Visual Accessibility, Scaling and Motion

## Theme and contrast

Measure the *rendered foreground/background combination* in each theme. Design tokens can be valid brand colors without being valid body-text colors.

For Web WCAG 2.2 AA:
- ordinary text: **at least 4.5:1**;
- large text: **at least 3:1**;
- essential control boundaries, state indicators and meaningful graphic elements: **at least 3:1**, where applicable;
- focus indicator must be perceivable; the enhanced focus-appearance criterion is AAA, not AA.

Use `DESIGN_TOKEN_CONTRAST_AUDIT.md` for tested Part 4A color pairings. Do not assume all branded colors or muted text tokens can be used as small text on all backgrounds.

## Text and zoom

- Transcript body should start from RimV's readable body-large token, then adapt to user scaling.
- Allow 200% text scaling on the web without losing functionality (WCAG 1.4.4 AA).
- Test reflow at a 320 CSS px equivalent width; don't force horizontal scrolling for ordinary prose or control layout (1.4.10 AA, with exceptions).
- Test custom text spacing: line 1.5×, paragraph 2×, letter 0.12×, word 0.16× where criterion applies (1.4.12 AA).
- On iOS, use semantic font styles and Dynamic Type; on desktop, respect toolkit/system text scaling.

## Forced colors and high contrast

- Web: verify Windows forced-colors mode. Don't depend on brand-background fills to identify states; use text, border and system colors where necessary.
- Windows native: use high-contrast-aware resources and UI Automation semantics.
- Apple: verify Increase Contrast, Larger Text and system theme.
- Linux: respect toolkit palette, high-contrast theme and font scaling.
- Keep visible status labels in all themes.

## Motion

- Respect `prefers-reduced-motion` and platform reduction preferences.
- Replace decorative pulses and shimmer with static state indication when motion is reduced.
- A waveform may communicate live signal, but `Recording` text and Stop control remain available when animation is absent.
- Avoid unexpected content movement during incremental transcription.
- Do not use flashing status indicators near photosensitive thresholds.

## Layout density

- Maintain focus visibility at enlarged text size, narrow windows and browser zoom.
- Stack controls before shrinking labels to unreadable sizes.
- Avoid clipping error guidance and button labels.
- Do not use low-contrast placeholder text as the only label.
