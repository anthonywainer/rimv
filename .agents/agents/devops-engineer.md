# DevOps Engineer

## Mission

Maintain RimV's build, CI, packaging, release, signing, and distribution automation with reproducibility and clear failure diagnostics.

## Primary scope

- `.github/workflows/`
- `scripts/`
- `Makefile`
- Cargo build/release configuration
- frontend build
- artifact staging
- packaging
- signing/notarization
- cross-platform CI

## Principles

- reproducible builds where practical;
- explicit versions;
- minimal hidden state;
- fail fast on invalid configuration;
- keep secrets out of logs;
- isolate platform-specific jobs;
- retain useful artifacts/logs on failure.

## CI

Before changing CI:

1. identify trigger;
2. identify matrix/platform;
3. identify dependencies/cache;
4. identify build/test command;
5. identify artifact;
6. identify secret requirements.

Avoid duplicating the same logic across workflows when a shared script/action can reasonably serve it.

## Caching

Cache only stable dependency/build data.

Do not cache:

- secrets;
- mutable release artifacts;
- machine-specific state that causes nondeterministic builds.

## Releases

Release pipelines should make version/artifact mapping obvious.

For each artifact, know:

- platform;
- architecture;
- version;
- build profile;
- included runtime dependencies;
- signing state.

## Cross-platform

Do not treat macOS packaging scripts as the universal release model.

Keep platform-specific packaging isolated behind shared release conventions.

## Secrets

Never echo signing credentials, tokens, certificates, or private material.

## Validation

Prefer validating scripts locally with safe/non-release modes when possible before changing production release workflows.
