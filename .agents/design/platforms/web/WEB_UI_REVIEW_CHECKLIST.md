# RimV Web UI Review Checklist

## Design system

- [ ] RimV tokens used
- [ ] spacing/radii consistent
- [ ] light/dark themes work
- [ ] semantic colors used correctly
- [ ] AI/live/error states distinct

## Responsive

- [ ] large desktop works
- [ ] medium layout works
- [ ] narrow layout works
- [ ] no unnecessary horizontal scrolling
- [ ] primary content remains readable
- [ ] secondary panels collapse appropriately

## Interaction

- [ ] primary action obvious
- [ ] hover not required for critical actions
- [ ] keyboard works
- [ ] focus visible
- [ ] dialogs restore focus
- [ ] loading state visible
- [ ] error/retry flow exists

## Recording/transcription

- [ ] idle state
- [ ] permission state
- [ ] preparing state
- [ ] listening/recording state
- [ ] transcribing/processing state
- [ ] completion state
- [ ] recoverable error state
- [ ] device changes handled where relevant

## Transcript

- [ ] selectable
- [ ] copy works
- [ ] long text remains readable
- [ ] user scroll position respected
- [ ] live updates do not steal focus
- [ ] partial/final distinction clear if applicable

## Accessibility

- [ ] semantic HTML
- [ ] labels
- [ ] keyboard
- [ ] focus
- [ ] contrast
- [ ] zoom
- [ ] reduced motion
- [ ] screen-reader state

## Engineering

- [ ] no unnecessary framework introduced
- [ ] no unnecessary dependency introduced
- [ ] TypeScript types are narrow
- [ ] unsafe HTML injection avoided
- [ ] repeated Tailwind patterns controlled
