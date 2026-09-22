# Apple UI Review Checklist

Use for meaningful RimV macOS/iOS UI reviews.

## Shared

- [ ] RimV design system followed
- [ ] primary state is obvious
- [ ] primary action is obvious
- [ ] live/AI/error semantics are correct
- [ ] light mode works
- [ ] dark mode works
- [ ] color is not the only indicator
- [ ] loading state exists where needed
- [ ] empty state exists where needed
- [ ] error/recovery state exists
- [ ] permission-denied flow exists where relevant
- [ ] native controls used where suitable
- [ ] modal presentation is justified
- [ ] accessibility labels/values are meaningful
- [ ] reduced motion considered

## macOS

- [ ] window resizes sensibly
- [ ] minimum window size is usable
- [ ] keyboard navigation works
- [ ] standard shortcuts are respected
- [ ] menus follow macOS conventions
- [ ] status/menu-bar behavior is clear if applicable
- [ ] hover is not required to discover critical actions
- [ ] transcript selection/copy works where relevant

## iOS

- [ ] touch targets are large enough
- [ ] Dynamic Type works
- [ ] large accessibility text does not break layout
- [ ] safe areas respected
- [ ] gestures have accessible alternatives where needed
- [ ] interruption/background state is accurate
- [ ] sheets are not excessively nested
- [ ] system share/navigation patterns used appropriately

## VoiceOver

- [ ] control names describe purpose
- [ ] current recording/listening state is announced
- [ ] progress values are meaningful
- [ ] custom controls expose correct role/state
- [ ] focus order is logical
