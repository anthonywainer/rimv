# Component and Interaction Parity Matrix

Use this as a **product behavior and accessibility checklist**, not a requirement that every platform share the same control implementation. For each new feature, select the applicable rows and create or update a parity review using the template.

| Component | Shared contract | Apple | Windows | Linux | Web |
|---|---|---|---|---|---|
| Capture primary action | Start/stop label and true state; accessible | Native button, menu-bar shortcut if implemented | Native button, command/tray if implemented | Toolkit button; don't require tray | Semantic `button`, browser permission flow |
| Live indicator | Label/icon + `live` semantics | Toolbar/status, VoiceOver state | Status/automation properties | Toolkit status + screen-reader label | Text + icon; non-spammy live region |
| Device picker | Human-readable choices; active choice clear | System/native picker pattern | Native combo/menu | Toolkit/portal as applicable | Browser `enumerateDevices` only if supported/permitted |
| Model selection | Current model, readiness and size/progress if exposed | Form/sheet | Native settings/dialog | Toolkit preferences | Accessible select/dialog |
| Transcript viewer | Searchable/selectable/copyable when supported; partial vs final distinguished | Native text/list | Native text/list | Toolkit text view | Selectable semantic text; preserve scroll/focus |
| AI result | Clearly labeled derivative content; not confused with original | Native group/card | Native group/card | Toolkit group/card | Semantic section/card |
| Progress | Operation named; determinate when measurable | Native progress | ProgressBar/Ring | Toolkit progress | Native `<progress>` where appropriate |
| Toast/notification | Non-blocking feedback; relevant urgency | In-app/system when needed | In-app/system when needed | In-app/desktop service | In-page status, system notification only with consent |
| Error recovery | What happened + next action + details separately | Alert/sheet/status | Dialog/inline status | Toolkit dialog/inline | Inline alert/dialog |
| Settings | Same conceptual option with accurate availability | Apple Settings/forms | Windows-style settings | Toolkit preferences | Responsive settings view |
| Export/share | Format/content transparent; actual support indicated | Share/open/save UI | Windows file/share API | Portal/native file dialog | Download/share only if browser capability exists |
| Secondary navigation | Same discoverable destination labels | Sidebar/tabs/stack | Nav or commands | Toolkit nav | Responsive nav |

## 1. States per component

Every interactive component with async behavior needs relevant `idle`, `pending`, `success`, `failure`, `disabled` and `cancelled` treatment. Not every component needs every state; review intentionally, don't invent artificial states.

**Primary capture control:**
- Idle: `Start listening` (if supported and ready).
- Preparing: visible activity; prevent accidental double-start.
- Active: `Stop listening` is easy to find, including via keyboard/screen reader.
- Stopping: indicate requested stop vs actual confirmed stop.
- Error: explain and recover; never show green `live` after capture has stopped due to error.

**Model download:**
- Pending / measurable progress / verifying / ready / failed / cancelled.
- Cancellation and resume support only if the underlying implementation actually supports them.
- A model's presence on disk does not necessarily imply it is loaded/usable.

## 2. Required accessibility parity

- Every critical interactive feature is operable using supported keyboard/touch input and assistive technology.
- Icon-only controls have meaningful names.
- Focus is visible; dialogs manage focus according to host platform conventions.
- Progress and error state use text, role and accessible value, not color alone.
- Transcript live updates do not forcibly move focus or continuously reannounce complete transcript content.
- Unsupported actions are absent or clearly disabled with explanation; never be deceptive.

## 3. Platform differences to permit

- An iOS share sheet vs Windows save dialog vs Linux portal vs web file download.
- macOS menu-bar controls vs Windows tray vs Linux optional tray vs web in-page status.
- NavigationStack on iOS vs desktop sidebar; layout changes according to context.
- Native font metrics, control dimensions, gesture/shortcut vocabulary.

## 4. Minimum parity evidence

For an affected component, record:

1. supported targets and availability;
2. what state/data is shared;
3. deliberate platform UI differences;
4. accessibility method;
5. one screenshot or description per tested platform/state (images stored outside this instructions folder if large);
6. tested/not-tested status, date and open gaps.

Use `templates/PARITY_REVIEW.md`. A passing macOS demo is not evidence of Windows, Linux, iOS or web behavior.
