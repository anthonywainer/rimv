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
    (cd "$(dirname "$stage_dir")" && zip -rq "$archive" "$(basename "$stage_dir")" \
      -x '*/.DS_Store' -x '*/__MACOSX/*' -x '*/target/*' -x '*/resources/models/*' \
      -x '*.onnx' -x '*.bin' -x '*.tar.bz2')
    # `unzip -l` includes a header with the archive's own filesystem path.
    # That path normally contains `target`, which the content validation below
    # correctly rejects for entries but must not mistake for an archive member.
    unzip -Z1 "$archive" > "$archive.contents.txt"
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
