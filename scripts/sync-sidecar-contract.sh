#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORKSPACE="$(cd "$ROOT/.." && pwd)"
SOURCE="$ROOT/packages/protocol/contracts/sidecar.v1.json"
if [[ ! -f "$SOURCE" ]]; then
  echo "missing contract: $SOURCE" >&2
  exit 1
fi
TARGETS=(
  "$WORKSPACE/sveda-php-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-python-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-node-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-go-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-java-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-dotnet-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-ruby-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-laravel-sdk/contracts/sidecar.v1.json"
  "$WORKSPACE/sveda-android/core/src/test/resources/sidecar.v1.json"
  "$WORKSPACE/sveda-ios/SvedaTests/Fixtures/sidecar.v1.json"
)
for target in "${TARGETS[@]}"; do
  mkdir -p "$(dirname "$target")"
  cp "$SOURCE" "$target"
  echo "synced $target"
done
