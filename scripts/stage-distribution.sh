#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
kind=${1:-}
profile=${PROFILE:-release}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
stage_root=${STAGE_DIR:-"$root/target/distributions"}
cargo_bin=${CARGO:-cargo}
skip_build=${RIMV_STAGE_SKIP_BUILD:-0}

case "$kind" in
  capture)
    package=capture-cli
    built_binary=capture-cli
    staged_binary=rimv-capture
    capability='Microphone capture; platform system-audio capture where implemented. No ASR or model weights.'
    ;;
  transcribe)
    package=rimv
    built_binary=rimv
    staged_binary=rimv
    capability='Capture, VAD, ASR, and model management. Model weights are downloaded separately.'
    ;;
  server)
    package=rimv
    built_binary=rimv
    staged_binary=rimv
    capability='Experimental local API. Start with: rimv serve (binds to 127.0.0.1 by default). Model weights are separate.'
    ;;
  full)
    printf '%s\n' 'rimv Full is omitted from 0.1.0-beta staging: third-party model redistribution terms have not been verified.' >&2
    exit 2
    ;;
  *)
    printf '%s\n' 'usage: scripts/stage-distribution.sh capture|transcribe|server|full' >&2
    exit 64
    ;;
esac

case "$(uname -s)" in
  Darwin) platform=macos; executable_suffix= ; library_glob='*.dylib' ;;
  Linux) platform=linux; executable_suffix= ; library_glob='*.so*' ;;
  MINGW*|MSYS*|CYGWIN*) platform=windows; executable_suffix=.exe; library_glob='*.dll' ;;
  *) printf 'unsupported build host: %s\n' "$(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch=arm64 ;;
  x86_64|amd64) arch=x64 ;;
  *) arch=$(uname -m) ;;
esac
destination="$stage_root/rimv-$version-$kind-$platform-$arch"
if [ "$skip_build" != 1 ]; then
  "$cargo_bin" build --profile "$profile" --locked -p "$package"
fi
rm -rf "$destination"
mkdir -p "$destination"
cp "$root/target/$profile/$built_binary$executable_suffix" "$destination/$staged_binary$executable_suffix"
cp "$root/LICENSE" "$destination/LICENSE"
printf 'rimv %s\nDistribution: %s\n%s\n' "$version" "$kind" "$capability" > "$destination/DISTRIBUTION.txt"

case "$kind" in
  capture)
    cat > "$destination/README.md" <<'EOF'
# rimv Capture

This distribution records microphone audio and supported system audio. It does
not include speech-to-text, ASR runtimes, or model weights.

```sh
./rimv-capture --help
./rimv-capture devices
./rimv-capture mic --seconds 30 --output microphone.wav
./rimv-capture system --seconds 30 --output system.wav
./rimv-capture both --seconds 30 --output-dir recordings
```

On macOS, grant Microphone permission for microphone capture. System audio also
requires Screen & System Audio Recording permission in System Settings.
EOF
    ;;
  transcribe)
    cat > "$destination/README.md" <<'EOF'
# rimv Transcribe

This distribution captures audio and converts finalized speech to text. ASR
model weights are not bundled. Before listening, configure the Parakeet model
directory and Silero VAD model file:

```sh
export RIMV_PARAKEET_MODEL_DIR=/path/to/parakeet-model-directory
export RIMV_SILERO_VAD_MODEL=/path/to/silero_vad.onnx
./rimv doctor
./rimv models
```

```sh
./rimv --help
./rimv listen --mic
./rimv listen --system
./rimv listen --both
```

On macOS, grant Microphone permission for microphone capture. System audio also
requires Screen & System Audio Recording permission in System Settings.
EOF
    ;;
  server)
    cat > "$destination/README.md" <<'EOF'
# rimv Server (Experimental)

This distribution starts rimv's local WebSocket bridge for the experimental web
UI. It binds only to `127.0.0.1` by default and is not intended for remote or
production use.

```sh
./rimv --help
./rimv serve
./rimv serve --port 7878
```

ASR model weights are not bundled. Configure model paths before using any
transcription features through the local runtime.
EOF
    ;;
esac

if [ "$kind" != capture ]; then
  for library in "$root/target/$profile"/$library_glob; do
    [ -e "$library" ] || continue
    cp "$library" "$destination/"
  done
  if [ "$platform" = macos ] && ! otool -l "$destination/$staged_binary" | grep -Fq '@loader_path'; then
    install_name_tool -add_rpath @loader_path "$destination/$staged_binary"
  fi
fi

printf '%s\n' "$destination"
