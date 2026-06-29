#!/bin/bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="XSSH"
BINARY_NAME="xssh"
BUNDLE_ID="com.elonehoo.xssh"
VERSION="0.0.1"
MIN_SYSTEM_VERSION="12.0"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_BINARY="$APP_MACOS/$APP_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"
ICON_SOURCE="$ROOT_DIR/assets/app-icon/app-icon.icns"
ICON_NAME="app-icon.icns"

if [[ ! -f "$ICON_SOURCE" ]]; then
  echo "App icon not found: $ICON_SOURCE" >&2
  exit 1
fi

stop_existing_processes() {
  pkill -x "$APP_NAME" >/dev/null 2>&1 || true
  pkill -x "$BINARY_NAME" >/dev/null 2>&1 || true
  pkill -f "$APP_BINARY" >/dev/null 2>&1 || true

  for ((attempt = 1; attempt <= 20; attempt++)); do
    if ! pgrep -x "$APP_NAME" >/dev/null && ! pgrep -x "$BINARY_NAME" >/dev/null && ! pgrep -f "$APP_BINARY" >/dev/null; then
      return
    fi

    sleep 0.1
  done

  pkill -9 -x "$APP_NAME" >/dev/null 2>&1 || true
  pkill -9 -x "$BINARY_NAME" >/dev/null 2>&1 || true
  pkill -9 -f "$APP_BINARY" >/dev/null 2>&1 || true
}

if [[ "$MODE" != "--verify" && "$MODE" != "verify" ]]; then
  stop_existing_processes
fi

cargo build

BUILD_BINARY="$ROOT_DIR/target/debug/$BINARY_NAME"
if [[ ! -x "$BUILD_BINARY" ]]; then
  echo "Built binary not found: $BUILD_BINARY" >&2
  exit 1
fi

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES"
cp "$BUILD_BINARY" "$APP_BINARY"
cp "$ICON_SOURCE" "$APP_RESOURCES/$ICON_NAME"
chmod +x "$APP_BINARY"

cat >"$INFO_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>English</string>
  <key>CFBundleDisplayName</key>
  <string>$APP_NAME</string>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIconFile</key>
  <string>$ICON_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>$MIN_SYSTEM_VERSION</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

plutil -lint "$INFO_PLIST" >/dev/null

run_app() {
  "$APP_BINARY"
}

verify_app() {
  test -x "$APP_BINARY"
  test -f "$APP_RESOURCES/$ICON_NAME"
  plutil -lint "$INFO_PLIST" >/dev/null
  "$BUILD_BINARY" --migrate-only >/dev/null
}

case "$MODE" in
  run)
    run_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    run_app
    ;;
  --telemetry|telemetry)
    run_app
    ;;
  --verify|verify)
    verify_app
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
