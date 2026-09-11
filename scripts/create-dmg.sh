#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=${VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)}
app=${1:-"$root/target/release/rimv.app"}
output=${2:-"$root/target/rimv-$version-macOS.dmg"}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -R "$app" "$stage/rimv.app"
ln -s /Applications "$stage/Applications"
rm -f "$output"
hdiutil create -volname rimv -srcfolder "$stage" -ov -format UDZO "$output"
shasum -a 256 "$output" > "${output%.dmg}.sha256"
printf '%s\n' "$output"
