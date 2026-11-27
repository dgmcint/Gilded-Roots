#!/bin/bash
# Builds the Mac app with microphone permission built in.
set -e
cd "$(dirname "$0")"
command -v cargo-bundle >/dev/null 2>&1 || cargo install cargo-bundle
cargo bundle --release
APP="target/release/bundle/osx/Gilded Roots.app"
PLIST="$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Delete :NSMicrophoneUsageDescription" "$PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :NSMicrophoneUsageDescription string Gilded Roots records the family stories you choose to save." "$PLIST"
# Finder and iCloud can leave extra data on the files, which codesign refuses ("detritus")
xattr -cr "$APP"
codesign --force --deep -s - "$APP"
tccutil reset Microphone local.gildedroots 2>/dev/null || true
echo ""
echo "Done. Your app is here: $APP"
echo "Drag it to your Desktop or Applications, open it, and click Allow when macOS asks about the microphone."
echo "Opening it now..."
open "$APP"
