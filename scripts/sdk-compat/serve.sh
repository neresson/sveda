#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export SVEDA_ENV_FILE="${SVEDA_ENV_FILE:-/dev/null}"
export SVEDA_BIND="${SVEDA_BIND:-127.0.0.1:18787}"
export SVEDA_LLM_CLIENT="${SVEDA_LLM_CLIENT:-scripted}"
export SVEDA_TITLE_GENERATION_ENABLED="${SVEDA_TITLE_GENERATION_ENABLED:-false}"
export SVEDA_EMBED_ENABLED=true
export SVEDA_EMBED_HOST_API_KEY="${SVEDA_EMBED_HOST_API_KEY:-sveda-compat-host-key}"
export SVEDA_APP_KEY="${SVEDA_APP_KEY:-sveda-compat-app-key}"
export SVEDA_DATABASE_URL="${SVEDA_DATABASE_URL:-}"
export SVEDA_REDIS_URL="${SVEDA_REDIS_URL:-}"
export NO_PROXY="${NO_PROXY:-127.0.0.1,localhost,::1}"
export no_proxy="$NO_PROXY"
cd "$ROOT"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BIN="$TARGET_DIR/debug/sveda-server"
if [[ ! -x "$BIN" ]]; then
  cargo build -p sveda-server
fi
exec "$BIN"
