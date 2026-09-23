#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export SVEDA_ENV_FILE="${SVEDA_ENV_FILE:-/dev/null}"
export SVEDA_BIND="${SVEDA_BIND:-127.0.0.1:18787}"
export SVEDA_EMBED_ENABLED=true
export SVEDA_ADMIN_API_KEY="${SVEDA_ADMIN_API_KEY:-sveda-e2e-admin-key}"
export SVEDA_APP_KEY="${SVEDA_APP_KEY:-sveda-e2e-app-key}"
export SVEDA_ADMIN_DIST="${SVEDA_ADMIN_DIST:-$ROOT/apps/runtime/public/build}"
export SVEDA_DATABASE_URL="${SVEDA_DATABASE_URL:-postgres://sveda:sveda@127.0.0.1:5432/sveda}"
export SVEDA_REDIS_URL="${SVEDA_REDIS_URL:-redis://127.0.0.1:6379}"
export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost,::1}"
export no_proxy="$NO_PROXY"
cd "$ROOT"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BIN="$TARGET_DIR/debug/sveda-server"
if [[ -x "$BIN" ]]; then
  exec "$BIN"
fi
exec cargo run -p sveda-server
