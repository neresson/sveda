#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export SVEDA_ENV_FILE="${SVEDA_ENV_FILE:-/dev/null}"
export SVEDA_BIND="${SVEDA_BIND:-127.0.0.1:18787}"
export SVEDA_EMBED_ENABLED=true
export SVEDA_ADMIN_API_KEY="${SVEDA_ADMIN_API_KEY:-sveda-e2e-admin-key}"
export SVEDA_APP_KEY="${SVEDA_APP_KEY:-sveda-e2e-app-key}"
export SVEDA_ADMIN_DIST="${SVEDA_ADMIN_DIST:-$ROOT/apps/runtime/public/build}"
export SVEDA_DATABASE_URL=""
export SVEDA_REDIS_URL=""
export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost,::1}"
export no_proxy="$NO_PROXY"
if [[ -n "${HTTP_PROXY:-}${HTTPS_PROXY:-}${http_proxy:-}${https_proxy:-}" ]]; then
  echo "sveda-e2e: outbound HTTP_PROXY=set" >&2
else
  echo "sveda-e2e: outbound HTTP_PROXY=unset" >&2
fi
if [[ -n "${DEEPSEEK_API_KEY:-}" ]]; then
  echo "sveda-e2e: DEEPSEEK_API_KEY=set" >&2
else
  echo "sveda-e2e: DEEPSEEK_API_KEY=missing (live chat tests will skip)" >&2
fi
cd "$ROOT"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BIN="$TARGET_DIR/debug/sveda-server"
echo "sveda-e2e: starting $BIN on $SVEDA_BIND" >&2
if [[ -x "$BIN" ]]; then
  exec "$BIN"
fi
exec cargo run -p sveda-server
