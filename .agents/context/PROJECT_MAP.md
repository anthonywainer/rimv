# RimV Project Map

This is a navigation aid, not a replacement for the source code.

Descriptions should be updated as RimV architecture evolves.

## Workspace root

- `Cargo.toml` — Rust workspace definition
- `Cargo.lock` — resolved Rust dependencies
- `Makefile` — project automation
- `README.md` — public project documentation
- `RELEASES.md` — release information
- `.cargo/config.toml` — Cargo configuration
- `.github/workflows/` — CI and release workflows
- `scripts/` — packaging and distribution tooling
- `resources/` — models, specifications, media, and project resources
- `vendor/` — vendored third-party source
- `target/` — generated Rust build output; ignore by default

## Core crates

### `crates/audio-core`
Shared audio-domain functionality.

### `crates/audio-capture`
Audio input/capture functionality.

### `crates/speech-transcription`
Speech-to-text / transcription functionality.

### `crates/whisper-models`
Whisper model integration.

### `crates/model-manager`
Model management and lifecycle.

### `crates/engine-runtime`
Runtime/execution layer for the engine.

### `crates/engine-protocol`
Shared protocol/contracts used for communication.

### `crates/system-profile`
System/device capability or environment profiling.

## Applications

### `apps/capture-cli`
Command-line capture application.

### `apps/transcribe-cli`
Command-line transcription application.

### `apps/rimv-cli`
General RimV CLI.

### `apps/engine-cli`
CLI around the engine/runtime.

### `apps/engine-web`
Web-facing engine integration.

### `apps/web`
Browser UI.

Current frontend toolchain:

- TypeScript
- Vite
- Tailwind CSS
- PostCSS
- Node test runner

No React/Vue/Svelte dependency should be assumed unless the package changes.

### `apps/menu-bar`
Native menu-bar application and platform integration.

## Resources

### `resources/specs/`
Versioned product/technical specifications.

### `resources/models/`
Local model files.

### `resources/audio/`
Audio fixtures/examples.

## Scripts

Packaging/release scripts currently include macOS-specific tooling. Cross-platform guidance should not assume macOS is the only target.

## Navigation hints

For audio/transcription issues, begin with:

- `audio-core`
- `audio-capture`
- `speech-transcription`
- `whisper-models`
- `model-manager`

For engine/message issues, begin with:

- `engine-protocol`
- `engine-runtime`
- relevant app consumer

For browser UI issues, begin with:

- `apps/web/src`
- `apps/web/package.json`

For native desktop issues, begin with the platform-specific app/integration layer before changing core crates.

## Important

This map is intentionally concise. If a module's responsibility is unclear, verify it in its `Cargo.toml`, public types, and source before making architectural decisions.
