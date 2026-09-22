# RimV — AI Assistance Entry Point

RimV uses `.agents/` as the project-local source of AI development guidance.

## Default behavior

- Start with files, symbols, errors, or modules explicitly mentioned in the task.
- Search before opening large files.
- Read the smallest relevant source sections first.
- Preserve existing architecture and public contracts unless the task requires changes.
- Prefer focused edits over broad refactors.
- Run the smallest relevant validation first.
- Keep final responses concise.
- Do not load every agent, skill, design guide, or workflow automatically.

## Ignore by default

Unless directly relevant, do not inspect:

- `target/`
- `apps/web/node_modules/`
- generated distributions and binaries
- `.DS_Store`
- vendored third-party code under `vendor/`

## Routing

Use:

- `.agents/rules/ROUTING.md`
- `.agents/context/PROJECT_MAP.md`
- `.agents/INDEX.md`

Choose one primary specialist whenever possible.

### Common routing

- Rust/core/runtime/protocols → `.agents/agents/rust-developer.md`
- Audio/capture/transcription/models → `.agents/agents/audio-engineer.md`
- macOS/iOS/Swift → `.agents/agents/swift-developer.md`
- Windows → `.agents/agents/windows-developer.md`
- Linux → `.agents/agents/linux-developer.md`
- Web/TypeScript/Vite/Tailwind → `.agents/agents/frontend-developer.md`
- UI/UX → `.agents/agents/ui-ux-designer.md`
- Architecture → `.agents/agents/software-architect.md`
- Debugging → `.agents/agents/debugger.md`
- Testing → `.agents/agents/test-engineer.md`
- Security/privacy → `.agents/agents/security-engineer.md`
- CI/release/packaging → `.agents/agents/devops-engineer.md`
- Performance/latency/memory → `.agents/agents/performance-engineer.md`
- Code review → `.agents/agents/reviewer.md`

## Skills

Load a skill only when the task needs that procedure.

See `.agents/INDEX.md` for the available skill set and design/UX guidance.

## UI tasks

For UI work:

1. use `.agents/design/RIMV_DESIGN_SYSTEM.md`;
2. load only the target platform guidance;
3. load the relevant UI skill;
4. use usability/accessibility review skills only when needed.

## Cross-component changes

Before changing a shared contract:

1. identify producer and consumer;
2. inspect the contract first;
3. preserve compatibility unless intentionally breaking it;
4. validate the narrowest integration path.

Use Software Architect guidance for new crates, public protocol changes, platform abstractions, or major dependency-direction changes.

## Completion

At completion:

- state success/failure;
- mention important changed files only when useful;
- report failed checks or unresolved issues;
- do not repeat the diff;
- do not produce a long summary unless requested.
