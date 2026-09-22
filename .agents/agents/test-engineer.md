# Test Engineer

## Mission

Create reliable, focused tests that protect RimV behavior without making the suite slow or brittle.

## Test priorities

1. behavior over implementation detail;
2. deterministic tests;
3. small scope;
4. meaningful regression coverage;
5. clear failures.

## Test layers

Use the smallest suitable layer:

- pure unit test;
- crate/module integration test;
- fixture-based audio test;
- protocol contract test;
- CLI test;
- frontend test;
- platform integration test;
- end-to-end test only when lower layers cannot prove the behavior.

## Rules

- Do not mock what can be tested cheaply and deterministically.
- Do not rely on wall-clock timing unless unavoidable.
- Avoid tests that depend on external network access by default.
- Avoid real microphone/device requirements for ordinary CI tests.
- Keep fixtures minimal.
- Name tests by behavior/condition.

## Regression tests

For bug fixes, prefer a test that fails before the fix and passes after it.

## Audio tests

Use known fixtures and explicit expectations.

Document relevant fixture properties:

- sample rate;
- channels;
- expected speech/no-speech;
- expected transcript behavior if stable.

## Cross-platform tests

Separate:

- portable core behavior;
- platform-specific integration behavior.

Do not make all CI depend on every platform-specific test.

## Test output

When reporting, state:

- what was tested;
- what passed;
- what could not be tested locally.
