# Forms, Inputs, Selectors and Settings

Use the central design system for tokens; do not invent colors or spacing per screen.

## Input dimensions

| Control | Desktop | Touch-oriented |
|---|---|---|
| Text input/select | 40px high, 12px radius, 12–14px horizontal padding | At least 44–48px high; native input semantics |
| Segmented control | 32–36px with shared surrounding surface | Native equivalent with adequate hit target |
| Toggle | Native convention | Native convention |
| Status chip | 26px high; 10–12px horizontal padding | Can expand for accessible tap target if interactive |

## Settings hierarchy

1. Page/section heading.
2. Short description only if meaning is not obvious.
3. Persistent label.
4. Control or selected value.
5. Helper text and error/recovery message when needed.

Never replace a persistent label with placeholder text for a field whose purpose may be forgotten after entry. Group related settings visually with `surface-secondary`, `border`, 12–16px row gaps.

## Form states

- `idle`: clear input surface and stable label;
- `hover`: subtle border change;
- `focus`: 3px or system-equivalent visible ring using `focus` token;
- `invalid`: `error` outline plus human-readable helper text; show correction;
- `loading`: indicate fetch/save state without clearing user input;
- `saved`: concise confirmation that does not interrupt keyboard use;
- `disabled`: give reason when a feature is temporarily unavailable.

## Selection and errors

- A settings selection should not unexpectedly perform irreversible work.
- Destructive settings need explicit confirmation when recovery is not possible.
- Preserve user input on recoverable validation/network/backend errors.
- Include a visible default/selected state, never indicate it by color alone.
- Avoid platform-specific drop-down simulation when native controls meet the requirement.

## RimV examples

- Microphone/device selector: show current and missing/disconnected devices.
- Transcription language selector: distinguish Auto from an explicit language.
- Model selector: include model availability/readiness; avoid selecting an unavailable model silently.
- Theme selector: System / Light / Dark with a clearly indicated selection.

Native platform conventions are defined in Part 4B; detailed permission UX is Part 4C.
