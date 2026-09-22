# RimV AI Assistance Index

This is the navigation index for the `.aiassistance/` system.

## Foundation

- `README.md`
- `rules/ROUTING.md`
- `rules/CONTEXT_EFFICIENCY.md`
- `context/PROJECT_MAP.md`
- `workflows/TASK_EXECUTION.md`

## Specialist agents

Located in `agents/`.

Primary roles include:

- Rust Developer
- Audio Engineer
- Swift Developer
- Windows Developer
- Linux Developer
- Frontend Developer
- UI/UX Designer
- Software Architect
- Debugger
- Test Engineer
- Security Engineer
- DevOps Engineer
- Performance Engineer
- Reviewer

## Engineering skills

Located in `skills/`.

Use only the skill relevant to the current task. Typical categories include:

- clean code
- SOLID/design quality
- Rust
- concurrency
- audio processing
- transcription
- model inference
- Swift
- Windows
- Linux
- TypeScript/frontend
- debugging
- testing
- performance
- security
- accessibility
- API/contracts
- CI/CD
- cross-platform development

## Design

Canonical visual source:

- `design/RIMV_DESIGN_SYSTEM.md`

Platform-specific guidance:

- `design/platforms/apple/`
- `design/platforms/windows/`
- `design/platforms/linux/`
- `design/platforms/web/`

Cross-platform design guidance is also stored under `design/`.

## UX and usability

- `ux/usability-heuristics/`
- `skills/usability-heuristics/`
- `workflows/USABILITY_AUDIT.md`

## Accessibility

Use the accessibility guidance and audit skill included in this repository.

Do not treat heuristic usability review and accessibility conformance as the same activity.

## UX testing and review

Use the UX testing and review skills/workflows when evaluating real user flows, release readiness, or observed usability evidence.

## Loading rule

Do not load this entire tree for every task.

Recommended order:

1. root `AGENTS.md`
2. `rules/ROUTING.md`
3. one specialist
4. one relevant skill/workflow
5. platform/design/context files only if needed
