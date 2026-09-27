# Native Windows manual acceptance checklist

Use this checklist to record interactive Windows acceptance results. This report is optional and is not a release-workflow gate. If you create a report at `docs/native-windows-validation/<tag>.md`, record the Windows version, hardware, installer checksum and observed result for each item:

```text
installer: PASS
tray: PASS
keyboard_scaling: PASS
microphone: PASS
system_audio: PASS
both_audio: PASS
recordings_exports: PASS
shutdown: PASS
```

Record clean install and uninstall, first launch, tray popup and selectors, keyboard/Narrator and scaling, real Microphone/System/Both capture on Windows hardware, recording persistence and TXT/JSON exports, and graceful Quit. Include the tested commit SHA, CI run URL containing the native package jobs, install path, model version and any remaining limitations. Do not mark a check `PASS` based only on a headless runner or a successful build.

The release workflow separately requires a successful main-branch CI run for the tagged commit and downloads its precise validated installer. See [Windows installation instructions](../windows-native-installation.md).
