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
