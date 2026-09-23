#!/usr/bin/env bash
set -euo pipefail
BASE_URL="${SVEDA_BASE_URL:-http://127.0.0.1:18787}"
for _ in $(seq 1 60); do
  if curl -fsS "${BASE_URL%/}/sveda/ready" >/dev/null 2>&1; then
    exit 0
  fi
  sleep 1
done
echo "sveda-sdk-compat: server not ready at ${BASE_URL}" >&2
exit 1
