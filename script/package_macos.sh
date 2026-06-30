#!/bin/bash
set -euo pipefail

APP_NAME="${APP_NAME:-XSSH}"
BINARY_NAME="${BINARY_NAME:-xssh}"
BUNDLE_ID="${BUNDLE_ID:-com.elonehoo.xssh}"
VERSION="${VERSION:-}"
MIN_SYSTEM_VERSION="${MIN_SYSTEM_VERSION:-12.0}"
PROFILE="${PROFILE:-release}"
TARGET="${TARGET:-}"
SIGN_IDENTITY="${SIGN_IDENTITY:-}"
NOTARIZE="${NOTARIZE:-0}"
ARTIFACT_SUFFIX="${ARTIFACT_SUFFIX:-macos}"
SPARKLE_VERSION="${SPARKLE_VERSION:-2.9.3}"
SPARKLE_PUBLIC_ED_KEY="${SPARKLE_PUBLIC_ED_KEY:-ThlCLJntIC8sne/cyqx6y0ZbAk1CmAaKXYVXUCO94V4=}"
SPARKLE_FEED_URL="${SPARKLE_FEED_URL:-https://github.com/elonehoo/xssh/releases/latest/download/appcast.xml}"
SPARKLE_FRAMEWORK_PATH="${SPARKLE_FRAMEWORK_PATH:-}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/release"
SPARKLE_CACHE_DIR="$ROOT_DIR/target/sparkle"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_FRAMEWORKS="$APP_CONTENTS/Frameworks"
APP_BINARY="$APP_MACOS/$APP_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"
ICON_SOURCE="$ROOT_DIR/assets/app-icon/app-icon.icns"
ICON_NAME="app-icon.icns"

if [[ -z "$VERSION" ]]; then
  VERSION="$(awk -F '"' '/^version =/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
fi

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required environment variable: $name" >&2
    exit 1
  fi
}

resolve_sparkle_framework() {
  if [[ -n "$SPARKLE_FRAMEWORK_PATH" ]]; then
    if [[ ! -d "$SPARKLE_FRAMEWORK_PATH" ]]; then
      echo "Sparkle framework not found: $SPARKLE_FRAMEWORK_PATH" >&2
      exit 1
    fi
    printf "%s\n" "$SPARKLE_FRAMEWORK_PATH"
    return
  fi

  local sparkle_root="$SPARKLE_CACHE_DIR/Sparkle-$SPARKLE_VERSION"
  local sparkle_framework="$sparkle_root/Sparkle.framework"
  if [[ ! -d "$sparkle_framework" ]]; then
    local archive="$SPARKLE_CACHE_DIR/Sparkle-$SPARKLE_VERSION.tar.xz"
    local url="https://github.com/sparkle-project/Sparkle/releases/download/$SPARKLE_VERSION/Sparkle-$SPARKLE_VERSION.tar.xz"
    rm -rf "$sparkle_root"
    mkdir -p "$sparkle_root" "$SPARKLE_CACHE_DIR"
    curl -fsSL "$url" -o "$archive"
    tar -xf "$archive" -C "$sparkle_root" Sparkle.framework bin/generate_appcast
  fi

  printf "%s\n" "$sparkle_framework"
}

embed_sparkle_framework() {
  local sparkle_framework
  sparkle_framework="$(resolve_sparkle_framework)"
  mkdir -p "$APP_FRAMEWORKS"
  rm -rf "$APP_FRAMEWORKS/Sparkle.framework"
  ditto "$sparkle_framework" "$APP_FRAMEWORKS/Sparkle.framework"
}

sign_bundle_if_exists() {
  local path="$1"
  if [[ -e "$path" ]]; then
    codesign --force --timestamp --options runtime --sign "$SIGN_IDENTITY" "$path"
  fi
}

if [[ "$PROFILE" != "debug" && "$PROFILE" != "release" ]]; then
  echo "PROFILE must be debug or release" >&2
  exit 2
fi

if [[ ! -f "$ICON_SOURCE" ]]; then
  echo "App icon not found: $ICON_SOURCE" >&2
  exit 1
fi

BUILD_ARGS=(build --locked)
if [[ "$PROFILE" == "release" ]]; then
  BUILD_ARGS+=(--release)
fi
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
fi

cargo "${BUILD_ARGS[@]}"

if [[ -n "$TARGET" ]]; then
  BUILD_BINARY="$ROOT_DIR/target/$TARGET/$PROFILE/$BINARY_NAME"
else
  BUILD_BINARY="$ROOT_DIR/target/$PROFILE/$BINARY_NAME"
fi

if [[ ! -x "$BUILD_BINARY" ]]; then
  echo "Built binary not found: $BUILD_BINARY" >&2
  exit 1
fi

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES" "$APP_FRAMEWORKS"
cp "$BUILD_BINARY" "$APP_BINARY"
cp "$ICON_SOURCE" "$APP_RESOURCES/$ICON_NAME"
chmod +x "$APP_BINARY"
embed_sparkle_framework

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
  <key>CFBundleSupportedPlatforms</key>
  <array>
    <string>MacOSX</string>
  </array>
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
  <key>SUEnableAutomaticChecks</key>
  <true/>
  <key>SUFeedURL</key>
  <string>$SPARKLE_FEED_URL</string>
  <key>SUPublicEDKey</key>
  <string>$SPARKLE_PUBLIC_ED_KEY</string>
</dict>
</plist>
PLIST

plutil -lint "$INFO_PLIST" >/dev/null
"$BUILD_BINARY" --migrate-only >/dev/null

if [[ -n "$SIGN_IDENTITY" ]]; then
  sign_bundle_if_exists "$APP_FRAMEWORKS/Sparkle.framework/Versions/B/Autoupdate"
  sign_bundle_if_exists "$APP_FRAMEWORKS/Sparkle.framework/Versions/B/Updater.app"
  sign_bundle_if_exists "$APP_FRAMEWORKS/Sparkle.framework/Versions/B/XPCServices/Downloader.xpc"
  sign_bundle_if_exists "$APP_FRAMEWORKS/Sparkle.framework/Versions/B/XPCServices/Installer.xpc"
  sign_bundle_if_exists "$APP_FRAMEWORKS/Sparkle.framework"
  codesign --force --timestamp --options runtime --sign "$SIGN_IDENTITY" "$APP_BINARY"
  codesign --force --timestamp --options runtime --sign "$SIGN_IDENTITY" "$APP_BUNDLE"
  codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
else
  echo "SIGN_IDENTITY is empty; skipping code signing"
fi

if [[ "$NOTARIZE" == "1" ]]; then
  if [[ -z "$SIGN_IDENTITY" ]]; then
    echo "NOTARIZE=1 requires SIGN_IDENTITY" >&2
    exit 1
  fi

  require_env APPLE_ID
  require_env APPLE_APP_SPECIFIC_PASSWORD
  require_env APPLE_TEAM_ID

  NOTARY_ZIP="$DIST_DIR/$APP_NAME-$VERSION-notary.zip"
  rm -f "$NOTARY_ZIP"
  ditto -c -k --keepParent "$APP_BUNDLE" "$NOTARY_ZIP"
  xcrun notarytool submit "$NOTARY_ZIP" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$APP_BUNDLE"
  xcrun stapler validate "$APP_BUNDLE"
  rm -f "$NOTARY_ZIP"
fi

DMG_PATH="$DIST_DIR/$APP_NAME-$VERSION-$ARTIFACT_SUFFIX.dmg"
CHECKSUM_PATH="$DMG_PATH.sha256"
rm -f "$DMG_PATH" "$CHECKSUM_PATH"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$APP_BUNDLE" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

if [[ -n "$SIGN_IDENTITY" ]]; then
  codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG_PATH"
fi

if [[ "$NOTARIZE" == "1" ]]; then
  xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$DMG_PATH"
  xcrun stapler validate "$DMG_PATH"
fi

shasum -a 256 "$DMG_PATH" >"$CHECKSUM_PATH"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "app_bundle=$APP_BUNDLE"
    echo "dmg_path=$DMG_PATH"
    echo "checksum_path=$CHECKSUM_PATH"
  } >>"$GITHUB_OUTPUT"
fi

echo "App bundle: $APP_BUNDLE"
echo "Release DMG: $DMG_PATH"
echo "Checksum: $CHECKSUM_PATH"
