#!/usr/bin/env bash
#
# Builds WhisprCatch.app and a distributable .dmg for macOS (Apple Silicon).
#
#   packaging/macos/build-dmg.sh
#
# Output: dist/WhisprCatch.app and dist/WhisprCatch-<version>-arm64.dmg
#
# Signing:
#   - By default the app is ad-hoc signed (codesign -s -). It runs locally, but
#     Gatekeeper will warn on first open (right-click → Open) and each rebuild
#     resets granted TCC permissions.
#   - For a proper release, set:
#         export SIGN_ID="Developer ID Application: Your Name (TEAMID)"
#     and (to notarize) run scripts/notarize after this, or set NOTARIZE=1 with
#     AC_* credentials (see packaging/macos/README.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

APP_NAME="WhisprCatch"
BUNDLE_ID="com.whisprcatch.app"
BIN="target/release/whisper-catch"
DIST="dist"
SIGN_ID="${SIGN_ID:--}"                 # "-" = ad-hoc
ENTITLEMENTS="packaging/macos/entitlements.plist"

VERSION="$(awk -F'"' '/^version/ {print $2; exit}' Cargo.toml)"
[ -n "$VERSION" ] || { echo "could not read version from Cargo.toml"; exit 1; }

echo "==> WhisprCatch $VERSION — building release binary"
if [ ! -x "$BIN" ]; then
  cargo build --release -p whisper-catch
fi
ARCH="$(uname -m)"   # arm64 on Apple Silicon

echo "==> Rendering AppIcon.icns"
rm -rf "$DIST"
ICONSET="$DIST/AppIcon.iconset"
mkdir -p "$ICONSET"
SRC_ICON="assets/icon-512.png"
gen() { sips -z "$1" "$1" "$SRC_ICON" --out "$ICONSET/$2" >/dev/null; }
gen 16   icon_16x16.png
gen 32   icon_16x16@2x.png
gen 32   icon_32x32.png
gen 64   icon_32x32@2x.png
gen 128  icon_128x128.png
gen 256  icon_128x128@2x.png
gen 256  icon_256x256.png
gen 512  icon_256x256@2x.png
gen 512  icon_512x512.png
gen 1024 icon_512x512@2x.png
iconutil -c icns "$ICONSET" -o "$DIST/AppIcon.icns"

echo "==> Assembling $APP_NAME.app"
APP="$DIST/$APP_NAME.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/whisper-catch"
chmod +x "$APP/Contents/MacOS/whisper-catch"
cp "$DIST/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleExecutable</key><string>whisper-catch</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <!-- Menu-bar app: no Dock icon -->
    <key>LSUIElement</key><true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>WhisprCatch transcribes your speech on-device while you hold the dictation key. Audio never leaves your machine.</string>
</dict>
</plist>
PLIST

echo "==> Signing ($([ "$SIGN_ID" = "-" ] && echo ad-hoc || echo "$SIGN_ID"))"
CS_ARGS=(--force --timestamp --options runtime --entitlements "$ENTITLEMENTS")
[ "$SIGN_ID" = "-" ] && CS_ARGS=(--force --entitlements "$ENTITLEMENTS")   # ad-hoc: no timestamp/hardened
codesign "${CS_ARGS[@]}" --sign "$SIGN_ID" "$APP/Contents/MacOS/whisper-catch"
codesign "${CS_ARGS[@]}" --sign "$SIGN_ID" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP" || true

echo "==> Building .dmg"
STAGE="$DIST/dmg"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
DMG="$DIST/${APP_NAME}-${VERSION}-${ARCH}.dmg"
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE" "$ICONSET"

echo ""
echo "Done:"
echo "  app: $APP"
echo "  dmg: $DMG"
[ "$SIGN_ID" = "-" ] && echo "  note: ad-hoc signed — users open via right-click → Open the first time."
