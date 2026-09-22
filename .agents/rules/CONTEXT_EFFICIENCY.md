# Context Efficiency Rules

These rules are intended to keep AI-assisted development focused and reduce unnecessary context growth.

They are optimization guidelines, not hard limits.

## 1. Search before reading

Prefer:

1. exact file mentioned by the task;
2. symbol search;
3. direct dependency;
4. nearby tests;
5. broader repository search only if needed.

Avoid opening entire large files when a relevant function or type can be located first.

## 2. Read narrow sections first

When a match is found:

- inspect the surrounding function/type;
- inspect imports only when relevant;
- inspect callers/callees only as needed;
- expand outward only if the local context is insufficient.

## 3. Ignore generated content

Avoid reading:

- `target/`
- bundled distribution output;
- `node_modules/`
- generated lock/build artifacts;
- compiled libraries/binaries;
- `.DS_Store`.

Read them only when the task explicitly concerns generated output or packaging.

## 4. Avoid repeated reads

Do not re-open unchanged files unless:

- later edits changed them;
- a failed check points back to them;
- important context was missed.

## 5. Prefer repository maps

Use `.aiassistance/context/PROJECT_MAP.md` to identify likely modules instead of exploring every crate.

## 6. Keep tool output focused

Prefer:

- targeted test commands;
- targeted compiler output;
- filtered logs;
- exact symbol searches.

Avoid dumping full logs when the final lines or specific error are sufficient.

## 7. Test progressively

Validation order:

1. smallest relevant unit or package;
2. affected crate/app;
3. integration path;
4. whole workspace only when justified.

Do not repeatedly run expensive workspace-wide checks after every small edit.

## 8. Preserve thread focus

For unrelated tasks, start a fresh coding conversation when practical.

Long unrelated histories can make reasoning less focused even when automatic compaction is available.

## 9. Keep instruction files concise

Instruction files themselves consume context when loaded.

Therefore:

- avoid duplicate rules;
- reference the source of truth;
- keep routing files short;
- move detailed domain guidance into specialist files;
- load detailed files only when required.

## 10. Final response discipline

Default completion output should include only:

- success/failure status;
- important changed files if useful;
- failed checks;
- unresolved decisions.

Do not:

- repeat code already visible in the diff;
- restate the entire task;
- produce a detailed change summary unless requested.

## 11. Quality takes priority

Do not skip necessary context merely to reduce token usage.

If a change is unsafe without reading a dependency, contract, test, or platform rule, read it.
