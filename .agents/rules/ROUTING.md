# Task Routing Rules

Use this file to decide which guidance should be loaded for a task.

The goal is to avoid loading irrelevant instructions.

## 1. Direct tasks

Handle a task directly when it is:

- small;
- localized;
- low risk;
- clearly scoped;
- limited to one or two files;
- not architecture-sensitive.

Examples:

- rename a local variable;
- fix a typo;
- adjust one CSS class;
- fix one obvious compile error;
- update one test expectation.

Do not invoke multiple specialist roles for these tasks.

## 2. Specialist routing

When specialist agent files are available, route tasks according to the primary technical concern.

| Concern | Specialist |
|---|---|
| Rust crates, runtime, protocols, CLI | Rust Developer |
| Audio capture, transcription, VAD, models | Audio Engineer |
| macOS, iOS, Swift, SwiftUI, AppKit/UIKit | Swift Developer |
| Windows native integration/UI | Windows Developer |
| Linux integration/UI | Linux Developer |
| TypeScript, Vite, Tailwind, browser UI | Frontend Developer |
| Architecture and cross-component contracts | Software Architect |
| UI structure and interaction design | UI/UX Designer |
| CI, packaging, release automation | DevOps Engineer |
| Security-sensitive behavior | Security Engineer |
| Test strategy and regressions | Test Engineer |

Use the smallest number of specialists necessary.

## 3. Skill routing

Skills describe how to perform a task.

Examples:

- Clean Code → refactoring or maintainability work
- Debugging → uncertain root cause
- Testing → adding or repairing coverage
- Performance → profiling or latency work
- Accessibility → user-interface accessibility
- Usability Heuristics → UX review
- Cross-platform → platform parity or abstraction design

Do not load a skill merely because it exists.

## 4. Cross-component work

For tasks that cross multiple modules:

1. identify the contract between components;
2. inspect the contract first;
3. identify producer and consumer;
4. edit only the affected sides;
5. validate the narrowest integration path.

Examples of contracts:

- Rust structs/enums shared across crates;
- IPC messages;
- command-line arguments;
- JSON schemas;
- HTTP endpoints;
- native bridge interfaces;
- UI state models.

## 5. Design work

For UI work:

1. load the UI/UX specialist;
2. load the RimV design system;
3. load the relevant platform guidance;
4. load usability/accessibility skills only when needed.

Do not load macOS guidance for Windows-only work, and vice versa.

## 6. Escalation

Escalate to software architecture guidance when a change:

- introduces a new crate;
- changes crate boundaries;
- changes public APIs;
- changes IPC/protocol formats;
- affects multiple platforms;
- adds a new runtime dependency;
- changes persistence or compatibility behavior.

## 7. Review routing

Use review guidance for:

- concurrency changes;
- FFI;
- unsafe Rust;
- process execution;
- file-system security;
- update/install flows;
- privilege-sensitive code;
- large refactors;
- release-critical changes.

## 8. Avoid over-routing

Do not:

- invoke all specialists automatically;
- ask multiple agents to inspect the same files independently;
- run architecture review for trivial UI changes;
- load design guidance for audio internals;
- load platform guidance for unrelated backend work.
