# Linux UI Review Checklist

## Shared

- [ ] RimV design semantics preserved
- [ ] system theme respected
- [ ] system font respected
- [ ] primary action is obvious
- [ ] recording state unmistakable
- [ ] error/empty/loading states exist
- [ ] keyboard navigation works
- [ ] visible focus present
- [ ] color is not the only signal

## Environment

- [ ] toolkit-specific conventions followed
- [ ] no unsupported cross-DE assumptions
- [ ] Wayland implications considered
- [ ] X11 implications considered where supported
- [ ] global shortcut fallback exists if needed
- [ ] tray not required for essential functionality

## Audio

- [ ] current audio stack assumptions verified
- [ ] device disconnect handled
- [ ] default device change handled
- [ ] human-readable device names shown
- [ ] access failures produce recovery guidance

## Packaging/sandbox

- [ ] Flatpak/portal implications checked if relevant
- [ ] filesystem access compatible with packaging model
- [ ] notifications/dialogs work in target environment

## Scaling/accessibility

- [ ] HiDPI works
- [ ] fractional scaling considered
- [ ] screen reader semantics checked
- [ ] large fonts do not break layout
