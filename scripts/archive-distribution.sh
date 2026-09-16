#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
kind=${1:?usage: scripts/archive-distribution.sh capture|transcribe|server}
stage_dir=${2:?usage: scripts/archive-distribution.sh capture|transcribe|server path/to/staged-dir}
tag=${RELEASE_TAG:-}
archive_root=${ARCHIVE_DIR:-"$root/target/release-artifacts"}

if [ ! -d "$stage_dir" ]; then
  printf 'staged distribution does not exist: %s\n' "$stage_dir" >&2
  exit 1
fi

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
release_version=${tag:-"v$version"}
case "$release_version" in
  v*) ;;
  *) release_version="v$release_version" ;;
esac

name=$(basename "$stage_dir")
platform_arch=${name#rimv-$version-$kind-}
if [ "$platform_arch" = "$name" ]; then
  printf 'unexpected staged distribution name: %s\n' "$name" >&2
  exit 1
fi

artifact_base="rimv-$kind-$release_version-$platform_arch"
mkdir -p "$archive_root"

case "$platform_arch" in
  windows-*) archive="$archive_root/$artifact_base.zip" ;;
  *) archive="$archive_root/$artifact_base.tar.gz" ;;
esac

rm -f "$archive"

case "$archive" in
  *.zip)
    # GitHub's Windows runner provides PowerShell but not the Unix zip/unzip
    # utilities. Use the built-in ZIP APIs while keeping the staged directory
    # as the archive root, matching the tarball layout on macOS and Linux.
    stage_windows=$(cygpath -w "$stage_dir")
    archive_windows=$(cygpath -w "$archive")
    powershell.exe -NoProfile -NonInteractive -Command \
      "\$ErrorActionPreference = 'Stop'; Compress-Archive -LiteralPath '$stage_windows' -DestinationPath '$archive_windows' -Force"
    powershell.exe -NoProfile -NonInteractive -Command \
      "Add-Type -AssemblyName System.IO.Compression.FileSystem; [System.IO.Compression.ZipFile]::OpenRead('$archive_windows').Entries | ForEach-Object { \$_.FullName }" \
      > "$archive.contents.txt"
    ;;
  *.tar.gz)
    tar -czf "$archive" \
      --exclude='.DS_Store' \
      --exclude='__MACOSX' \
      --exclude='target' \
      --exclude='resources/models' \
      --exclude='*.onnx' \
      --exclude='*.bin' \
      --exclude='*.tar.bz2' \
      -C "$(dirname "$stage_dir")" "$(basename "$stage_dir")"
    tar -tzf "$archive" > "$archive.contents.txt"
    ;;
esac

if grep -E '(^|/)(target|resources/models)(/|$)|\.DS_Store$|(^|/)__MACOSX(/|$)|\.(onnx|bin)$|\.tar\.bz2$' "$archive.contents.txt"; then
  printf 'archive contains excluded release content: %s\n' "$archive" >&2
  exit 1
fi

printf '%s\n' "$archive"
