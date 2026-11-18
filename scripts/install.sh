#!/bin/sh
set -eu

REPOSITORY="muhsincaliskan/requestor"
ARCHIVE="Requestor-macos.zip"
APP="Requestor.app"
DOWNLOAD_URL="https://github.com/$REPOSITORY/releases/latest/download/$ARCHIVE"
QUARANTINE_ATTRIBUTE="com.apple.quarantine"

fail() {
  echo "requestor install: $1" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] || fail "this installer is for macOS; download other builds from https://github.com/$REPOSITORY/releases"

target_dir="/Applications"
[ -w "$target_dir" ] || target_dir="$HOME/Applications"
mkdir -p "$target_dir"

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

echo "Downloading $DOWNLOAD_URL"
curl -fL --progress-bar "$DOWNLOAD_URL" -o "$work_dir/$ARCHIVE" || fail "download failed"
ditto -x -k "$work_dir/$ARCHIVE" "$work_dir" || fail "could not unpack $ARCHIVE"
[ -d "$work_dir/$APP" ] || fail "$ARCHIVE does not contain $APP"

xattr -dr "$QUARANTINE_ATTRIBUTE" "$work_dir/$APP" 2>/dev/null || true
previous="$work_dir/previous-$APP"
if [ -d "$target_dir/$APP" ]; then
  mv "$target_dir/$APP" "$previous" || fail "could not replace $target_dir/$APP; quit Requestor and try again"
fi
if ! mv "$work_dir/$APP" "$target_dir/$APP"; then
  [ -d "$previous" ] && mv "$previous" "$target_dir/$APP"
  fail "could not install into $target_dir"
fi

echo "Installed $target_dir/$APP"
