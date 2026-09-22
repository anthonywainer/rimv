# RimV macOS Design Guidelines

Use together with:

- `RIMV_DESIGN_SYSTEM.md`
- `APPLE_PLATFORM_GUIDELINES.md`

## Product character

RimV on macOS should feel like a focused desktop utility, not a mobile or web interface placed inside a desktop window.

## Window structure

Choose the simplest structure that supports the product.

Possible structure:

```text
Window
├── Sidebar (only if multiple persistent sections justify it)
├── Main content
└── Inspector/context panel (only when useful)
```

Do not add a sidebar merely because macOS apps often have one.

## Toolbar

Good toolbar candidates:

- capture/listening control;
- device selection;
- search;
- relevant view switching;
- contextual actions.

Keep infrequent settings in Settings or menus.

## Menu bar application

If RimV runs as a menu-bar utility:

### Status item
It should communicate important state without becoming visually noisy.

### Popover/menu
Prioritize:

1. current status;
2. start/stop action;
3. recent/current transcript information if useful;
4. device/model state;
5. open main window;
6. settings;
7. quit.

Do not make destructive commands visually adjacent to frequent capture actions without separation.

## Keyboard

Desktop workflows should be efficient without the mouse.

Where appropriate, support shortcuts for:

- start/stop listening;
- focus search;
- open settings;
- show/hide main window;
- copy transcript;
- navigation.

Use familiar Apple shortcut conventions.

## Menus

Commands belong in standard menu locations when applicable.

Settings should follow the platform's Settings convention.

Avoid a custom in-content menu for commands already expected in the application menu bar.

## Pointer and hover

Hover may reveal secondary affordances, but essential actions must remain discoverable without hover.

Pointer targets should not be unnecessarily tiny.

## Resizing

Test:

- minimum supported width;
- comfortable default width;
- large window.

At narrow widths:

- collapse secondary regions;
- retain transcript readability;
- avoid horizontal scroll for ordinary UI.

## Transcripts

Long transcripts should:

- remain selectable;
- support copying;
- preserve reading position during live updates where possible;
- avoid stealing focus;
- avoid jumping unexpectedly when the user scrolls upward.

## System integration

Use native:

- open/save panels;
- notifications;
- settings;
- keyboard shortcuts;
- menu commands;
- status items;

instead of custom recreations.

## Accessibility

Test the primary capture-to-transcript flow using:

- keyboard only;
- VoiceOver;
- increased contrast;
- reduced motion.

Custom AppKit views must expose proper accessibility semantics.
