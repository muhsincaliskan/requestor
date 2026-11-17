#!/usr/bin/env bash
# Usage: scripts/bundle-macos.sh [target-triple...]
# Builds release binaries for the given targets (default: host), merges the two
# Apple targets into universal binaries with lipo, assembles
# target/bundle/Requestor.app, ad-hoc signs it and creates a .dmg when the
# macOS tools exist.
set -euo pipefail

APP_NAME="Requestor"
BUNDLE_IDENTIFIER="dev.requestor.app"
APP_BINARY="requestor-app"
CLI_BINARY="requestor"
APP_PACKAGE="requestor-app"
CLI_PACKAGE="requestor"
MIN_MACOS_VERSION="11.0"
ARM_TARGET="aarch64-apple-darwin"
INTEL_TARGET="x86_64-apple-darwin"
ROOT_MANIFEST="Cargo.toml"
APP_MANIFEST="crates/app/Cargo.toml"
BUNDLE_DIR="target/bundle"
UNIVERSAL_DIR="target/universal"
HOST_TARGET=""
DMG_VOLUME_NAME="$APP_NAME"

fail() {
  echo "error: $*" >&2
  exit 1
}

require_tool() {
  command -v "$1" >/dev/null 2>&1 || fail "required tool '$1' not found"
}

read_manifest_version() {
  sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

resolve_version() {
  local version
  version="$(read_manifest_version "$APP_MANIFEST")"
  if [ -z "$version" ]; then
    version="$(read_manifest_version "$ROOT_MANIFEST")"
  fi
  [ -n "$version" ] || fail "could not read a version from $APP_MANIFEST or $ROOT_MANIFEST"
  printf '%s' "$version"
}

build_target() {
  local target="$1"
  if [ -z "$target" ]; then
    cargo build --release --locked -p "$APP_PACKAGE" -p "$CLI_PACKAGE"
  else
    cargo build --release --locked --target "$target" -p "$APP_PACKAGE" -p "$CLI_PACKAGE"
  fi
}

release_dir() {
  if [ -z "$1" ]; then
    printf 'target/release'
  else
    printf 'target/%s/release' "$1"
  fi
}

has_target() {
  local wanted="$1"
  shift
  local candidate
  for candidate in "$@"; do
    [ "$candidate" = "$wanted" ] && return 0
  done
  return 1
}

make_universal() {
  require_tool lipo
  mkdir -p "$UNIVERSAL_DIR"
  local binary
  for binary in "$APP_BINARY" "$CLI_BINARY"; do
    lipo -create -output "$UNIVERSAL_DIR/$binary" \
      "$(release_dir "$ARM_TARGET")/$binary" \
      "$(release_dir "$INTEL_TARGET")/$binary"
  done
}

write_info_plist() {
  local plist="$1" version="$2"
  cat >"$plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_IDENTIFIER</string>
  <key>CFBundleExecutable</key><string>$APP_BINARY</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>$MIN_MACOS_VERSION</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
}

assemble_app() {
  local app_dir="$1" version="$2" source_dir="$3"
  rm -rf "$app_dir"
  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  cp "$source_dir/$APP_BINARY" "$app_dir/Contents/MacOS/$APP_BINARY"
  write_info_plist "$app_dir/Contents/Info.plist" "$version"
}

sign_app() {
  if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep -s - "$1"
  else
    echo "codesign not found, skipping ad-hoc signing" >&2
  fi
}

create_dmg() {
  local app_dir="$1" dmg_path="$2"
  if ! command -v hdiutil >/dev/null 2>&1; then
    echo "hdiutil not found, skipping .dmg" >&2
    return 0
  fi
  local staging
  staging="$(mktemp -d)"
  cp -R "$app_dir" "$staging/"
  ln -s /Applications "$staging/Applications"
  rm -f "$dmg_path"
  hdiutil create -volname "$DMG_VOLUME_NAME" -srcfolder "$staging" -ov -format UDZO "$dmg_path"
  rm -rf "$staging"
}

select_source_dir() {
  if has_target "$ARM_TARGET" "$@" && has_target "$INTEL_TARGET" "$@"; then
    make_universal
    printf '%s' "$UNIVERSAL_DIR"
  else
    release_dir "$1"
  fi
}

main() {
  require_tool cargo
  [ -f "$ROOT_MANIFEST" ] || fail "run this script from the repository root"

  local targets=("$@")
  if [ "${#targets[@]}" -eq 0 ]; then
    targets=("$HOST_TARGET")
  fi

  local target
  for target in "${targets[@]}"; do
    build_target "$target"
  done

  local source_dir version app_dir
  source_dir="$(select_source_dir "${targets[@]}")"
  version="$(resolve_version)"
  app_dir="$BUNDLE_DIR/$APP_NAME.app"
  mkdir -p "$BUNDLE_DIR"
  assemble_app "$app_dir" "$version" "$source_dir"
  cp "$source_dir/$CLI_BINARY" "$BUNDLE_DIR/$CLI_BINARY"
  sign_app "$app_dir"
  create_dmg "$app_dir" "$BUNDLE_DIR/$APP_NAME-$version.dmg"
  echo "Bundle ready: $app_dir"
}

main "$@"
