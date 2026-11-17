#!/usr/bin/env bash
# Usage: scripts/measure-ram.sh <tabs> <response-megabytes> <budget-megabytes>
# Opens <tabs> request tabs, loads a response of the given size into the active
# one, then reports the memory footprint. Exits 1 when the app or the fixture
# server fails, 2 above the hard ceiling and 0 otherwise; a footprint above the
# budget is reported as a warning.
set -uo pipefail

TABS="${1:-5}"
RESPONSE_MB="${2:-0}"
BUDGET_MB="${3:-120}"
CEILING_MB="${RAM_CEILING_MB:-200}"
SETTLE_SECONDS="${RAM_SETTLE_SECONDS:-10}"
PORT="${RAM_FIXTURE_PORT:-18473}"
BINARY="${REQUESTOR_BINARY:-target/release/requestor-app}"

if [ ! -x "$BINARY" ]; then
  cargo build --release --locked -p requestor-app || exit 1
fi

WORK="$(mktemp -d)"
SERVER_PID=""
APP_PID=""
cleanup() {
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
  rm -rf "$WORK"
}
trap cleanup EXIT

python3 scripts/ram_scenario.py "$WORK" "$TABS" "$RESPONSE_MB" "$PORT" || exit 1

APP_ARGS=()
if [ "$RESPONSE_MB" != "0" ]; then
  (cd "$WORK/fixture" && exec python3 -m http.server "$PORT" --bind 127.0.0.1) >/dev/null 2>&1 &
  SERVER_PID=$!
  FIXTURE_URL="http://127.0.0.1:$PORT/response.json"
  for _ in $(seq 1 50); do
    curl -sfI "$FIXTURE_URL" >/dev/null 2>&1 && break
    sleep 0.2
  done
  if ! curl -sfI "$FIXTURE_URL" >/dev/null 2>&1; then
    echo "::error::The response fixture server did not start"
    exit 1
  fi
  APP_ARGS+=(--send-active-tab)
fi

REQUESTOR_HOME="$WORK/home" "$BINARY" ${APP_ARGS[@]+"${APP_ARGS[@]}"} &
APP_PID=$!
sleep "$SETTLE_SECONDS"

if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "::error::Requestor exited before memory could be measured"
  exit 1
fi

case "$(uname -s)" in
  Darwin)
    FOOTPRINT_KB="$(footprint -p "$APP_PID" --format bytes 2>/dev/null \
      | awk '/Footprint:/ { for (i = 1; i < NF; i++) if ($i == "Footprint:") { printf "%d", $(i + 1) / 1024; exit } }')"
    ;;
  *)
    FOOTPRINT_KB="$(awk '/^Pss:/ { print $2; exit }' "/proc/$APP_PID/smaps_rollup" 2>/dev/null)"
    ;;
esac
if [ -z "${FOOTPRINT_KB:-}" ] || [ "$FOOTPRINT_KB" = "0" ]; then
  echo "::warning::Footprint was not available, falling back to resident set size"
  FOOTPRINT_KB="$(ps -o rss= -p "$APP_PID" | tr -d ' ')"
fi
FOOTPRINT_MB=$((FOOTPRINT_KB / 1024))

echo "tabs=$TABS response=${RESPONSE_MB}MB footprint=${FOOTPRINT_MB}MB budget=${BUDGET_MB}MB ceiling=${CEILING_MB}MB"
if [ "$FOOTPRINT_MB" -gt "$CEILING_MB" ]; then
  echo "::error::Memory footprint ${FOOTPRINT_MB} MB is above the ${CEILING_MB} MB ceiling"
  exit 2
fi
if [ "$FOOTPRINT_MB" -gt "$BUDGET_MB" ]; then
  echo "::warning::Memory footprint ${FOOTPRINT_MB} MB is above the ${BUDGET_MB} MB budget"
fi
exit 0
