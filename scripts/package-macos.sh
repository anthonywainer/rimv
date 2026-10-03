#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
profile=${PROFILE:-release}
app="$root/target/$profile/rimv.app"
cargo_bin=${CARGO:-cargo}
if ! command -v "$cargo_bin" >/dev/null 2>&1 && [ -x "$root/../.rust-tools/cargo/bin/cargo" ]; then
  cargo_bin="$root/../.rust-tools/cargo/bin/cargo"
fi
"$cargo_bin" build --profile "$profile" --locked -p rimv-menu-bar
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$root/target/$profile/rimv-menu-bar" "$app/Contents/MacOS/rimv-menu-bar"
# sherpa-onnx shared builds place their runtime dylibs beside the executable.
# Bundle those libraries so rimv.app remains self-contained outside Cargo's
# target directory. The loop is empty for configurations without dylibs.
for dylib in "$root/target/$profile"/*.dylib; do
  [ -e "$dylib" ] || continue
  cp "$dylib" "$app/Contents/MacOS/"
done
# sherpa-onnx uses @rpath install names. Cargo's link invocation does not retain
# the dependency's rpath on every macOS binary, so give the bundled executable
# an explicit search path beside itself.
install_name_tool -add_rpath @loader_path "$app/Contents/MacOS/rimv-menu-bar"
cp "$root/apps/menu-bar/Info.plist" "$app/Contents/Info.plist"
# Compile the repository asset catalog into the bundle so AppKit can resolve
# named images and named colors from the single source of truth.
xcrun actool "$root/resources/Assets.xcassets" \
  --compile "$app/Contents/Resources" \
  --platform macosx \
  --minimum-deployment-target 13.0 \
  --app-icon AppIcon \
  --accent-color AccentColor \
  --product-type com.apple.product-type.application \
  --output-partial-info-plist "$app/Contents/assetcatalog-info.plist"
/usr/libexec/PlistBuddy -c "Merge $app/Contents/assetcatalog-info.plist" "$app/Contents/Info.plist"
rm "$app/Contents/assetcatalog-info.plist"
# Free community releases use an ad-hoc signature only. It requires no Apple
# Developer certificate while keeping the bundle structurally valid.
codesign --force --deep --sign - --identifier com.rimv.app "$app"
printf '%s\n' "$app"
