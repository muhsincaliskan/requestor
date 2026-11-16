#!/usr/bin/env bash
set -euo pipefail

RESPONSE_MB="${1:-5}"
SETTLE_SECONDS="${2:-8}"
BINARY="${REQUESTOR_APP_BINARY:-target/release/requestor-app}"

if [[ ! -x "$BINARY" ]]; then
  cargo build --release -p requestor-app
fi

"$BINARY" --response-mb "$RESPONSE_MB" &
APP_PID=$!
trap 'kill "$APP_PID" 2>/dev/null || true' EXIT
sleep "$SETTLE_SECONDS"

case "$(uname -s)" in
  Darwin)
    echo "response=${RESPONSE_MB}MB (phys_footprint, the figure Activity Monitor shows)"
    footprint -p "$APP_PID" | grep -E "Footprint|phys_footprint" | head -3
    ;;
  Linux)
    echo "response=${RESPONSE_MB}MB (VmRSS and Pss)"
    grep -E "VmRSS|VmHWM" "/proc/$APP_PID/status"
    grep -E "^Pss:" "/proc/$APP_PID/smaps_rollup"
    ;;
esac
