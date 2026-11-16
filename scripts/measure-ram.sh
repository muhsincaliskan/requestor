#!/usr/bin/env bash
set -uo pipefail

RESPONSE_MB="${1:-5}"
SETTLE_SECONDS="${2:-8}"
BINARY="target/release/requestor-app"

cargo build --release -p requestor-app || exit 1

"$BINARY" --response-mb "$RESPONSE_MB" &
APP_PID=$!
trap 'kill "$APP_PID" 2>/dev/null || true' EXIT
sleep "$SETTLE_SECONDS"

echo "response=${RESPONSE_MB}MB settle=${SETTLE_SECONDS}s pid=${APP_PID}"
echo "rss_kb=$(ps -o rss= -p "$APP_PID" | tr -d ' ')"

case "$(uname -s)" in
  Darwin)
    footprint -p "$APP_PID" 2>&1 | head -20 || true
    ;;
  Linux)
    grep -E "VmRSS|VmHWM" "/proc/$APP_PID/status" || true
    grep -E "^Pss:" "/proc/$APP_PID/smaps_rollup" || true
    ;;
esac
