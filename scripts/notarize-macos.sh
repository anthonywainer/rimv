#!/bin/sh
set -eu

dmg=${1:?usage: notarize-macos.sh path/to/rimv.dmg}
: "${APPLE_ID:?Set APPLE_ID before notarizing}"
: "${APPLE_TEAM_ID:?Set APPLE_TEAM_ID before notarizing}"
: "${APPLE_APP_PASSWORD:?Set APPLE_APP_PASSWORD before notarizing}"

xcrun notarytool submit "$dmg" \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_PASSWORD" \
  --wait
xcrun stapler staple "$dmg"
