# Native Windows prerelease acceptance evidence

Before tagging a native Windows prerelease, commit `docs/native-windows-validation/<tag>.md` on the exact release commit. The release workflow requires these exact result lines, each backed by a short description of the Windows version, hardware, installer checksum and observed result:

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

After this report is committed, run the main-branch CI workflow for that exact commit. Create the new tag only when the CI run, including its native installer and fresh-runner validation jobs, passes. The release workflow rechecks that successful run ID and downloads its precise installer. See [Windows installation instructions](../windows-native-installation.md).
