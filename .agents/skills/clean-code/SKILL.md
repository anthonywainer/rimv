---
name: clean-code
description: "Use for maintainability refactors, code cleanup, naming and reducing duplication in RimV. Do not invoke for unrelated feature work or merely to reformat a file."
---

# Clean Code for RimV

## Goal
Make the code easier to change without altering externally observable behavior. Apply incrementally to the files in scope, never as a reason to rewrite unrelated modules.

## Procedure
1. **Specify behavior:** identify public signatures, protocol payloads, CLI flags, error behavior, persistence, and tests that must remain stable. Write down the observable behavior of the code you will change.
2. **Locate friction:** look for unclear names, mixed responsibilities, deep nesting, duplication of *policy* (not incidental syntax), implicit state, poorly located validation, and hidden side effects.
3. **Refactor one unit:** rename by intention, extract a cohesive function/type, simplify branches with early returns, or move logic to the correct owner. Follow current repository conventions.
4. **Protect boundaries:** keep `engine-protocol` as a contract, platform glue separate from domain logic, and audio hot paths free of unnecessary allocations or abstractions.
5. **Validate before continuing:** run the affected tests and compare outputs against the baseline. Review the diff for accidental public/API changes.

## Rust rules
- Use types and enums to express meaningful states rather than strings or magic flags.
- Prefer exhaustive matching where practical; do not hide unexpected states in a broad wildcard arm.
- Avoid `unwrap()` in fallible production paths; propagate or handle errors with appropriate context.
- Prefer iterators or loops based on clarity, not ideology. Do not force every loop into a chain.
- Keep ownership straightforward; cloning to silence the borrow checker can hide a design problem.
- Keep `unsafe` encapsulated and explain safety invariants.

## TypeScript/Swift rules
- Keep UI rendering separate from data acquisition and mutation.
- Use meaningful model names, stable state ownership, and explicit error handling.
- Avoid "helper" layers that only forward a call without protecting a real boundary.

## Code-smell decision table
| Smell | First response | Avoid |
|---|---|---|
| Repeated policy | Extract one shared implementation | Abstracting unrelated look-alikes |
| Long conditional | Name predicates; early return | Creating a complex strategy hierarchy by default |
| Giant module/view | Split by responsibility | Splitting every function into a new file |
| Ambiguous boolean | Use meaningful enum/type | Adding booleans until calls are unreadable |
| Magic constant | Named local or central token | Global constants for one-off values |

## Done when
- Behavior and relevant contracts are unchanged, unless changes were requested.
- The diff is smaller or meaningfully clearer; additional layers are justified.
- Relevant tests/checks pass and no known issue is disguised.

Related: `solid-design` for actual design-pressure problems; `testing` for behavioral protection.
