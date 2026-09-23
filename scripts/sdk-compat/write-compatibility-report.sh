#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
RESULTS_DIR="${1:-compat-results}"
if [[ ! -d "$RESULTS_DIR" && -d "$ROOT/../$RESULTS_DIR" ]]; then
  RESULTS_DIR="$ROOT/../$RESULTS_DIR"
fi
OUT="$ROOT/COMPATIBILITY.md"
SVEDA_VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
GENERATED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

{
  echo "# SDK compatibility matrix"
  echo
  echo "Generated: ${GENERATED_AT}"
  echo
  echo "Sveda workspace version: \`${SVEDA_VERSION}\`"
  echo
  echo "| SDK | Status |"
  echo "| --- | --- |"
  if [[ -d "$RESULTS_DIR" ]]; then
    while IFS= read -r line; do
      sdk="${line%%=*}"
      status="${line#*=}"
      echo "| ${sdk} | ${status} |"
    done < <(find "$RESULTS_DIR" -name 'matrix.env' -print0 | xargs -0 cat | sort -u)
  fi
  echo
  echo "Live smoke uses \`SVEDA_LLM_CLIENT=scripted\` and checks health, embed token mint, message, stream, and histories."
} > "$OUT"

echo "wrote $OUT"
