---
name: security-privacy
description: "Use for microphone/transcript privacy, filesystem/process operations, IPC, model downloads, unsafe/FFI, dependencies and release security. Do not run an unrelated full security audit for every typo."
---

# Security and Privacy Review

## Trust boundaries
Identify external inputs (audio, paths, protocol messages, model files, downloaded archives, CLI args), privileged operations, sensitive data and target outputs/logs.

## Review procedure
1. Draw the actual data flow: microphone → local buffers → transcript → optional persistence/export/network.
2. Verify whether audio or transcript ever leaves the device before claiming local-only behavior.
3. Identify permissions: who prompts, how denial is reported, and whether active capture is unmistakable.
4. Validate untrusted filenames, paths, lengths, serialized payloads and external process parameters at the boundary.
5. For downloaded models/native runtimes, review source trust, checksum/signature capability, partial-download handling, extraction paths and executable permissions.
6. Review `unsafe` and FFI contracts: buffer bounds, alignment, thread assumptions, ownership, callback lifetime, panic boundaries and error codes.
7. Keep secrets out of source, logs and CI artifacts. Never print signing keys or tokens.
8. Check recovery: partial write, update rollback, corrupted model, revoked permission, failed subprocess and cancellation.

## Privacy defaults
- Minimize unnecessary retention of raw audio/transcripts.
- Do not add diagnostic recording without explicit product requirements and visible consent.
- Use redacted metadata in logs by default.
- Provide accurate deletion/retention behavior when persistence is introduced.
- Avoid broad permissions when a narrow platform capability suffices.

## Process/filesystem rules
Use structured process arguments rather than interpolated shell commands with untrusted input. Follow OS-specific safe storage locations; use atomic writes where a partial file would be dangerous; consider symlink/path traversal where relevant.

## Risk-based verification
Test malformed payloads, denied permissions, corrupted archives and interrupted updates. For high-risk code, escalate to a dedicated reviewer and platform specialist. Do not claim formal audit or compliance based on checklist completion.

## Exit
Concrete trust boundaries are documented in code or change notes, necessary mitigations are implemented, sensitive logging avoided, and critical failure paths tested as available.
