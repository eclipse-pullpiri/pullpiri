#!/bin/bash

set -euo pipefail

LOG_FILE="build_results.log"
TMP_FILE="build_output.txt"
rm -f "$LOG_FILE" "$TMP_FILE"

echo "Running Cargo Build..." | tee -a "$LOG_FILE"

PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")
git config --global --add safe.directory "$PROJECT_ROOT"
cd "$PROJECT_ROOT"

MANIFESTS=(
  src/common/Cargo.toml
  src/agent/Cargo.toml
  src/player/Cargo.toml
  src/server/Cargo.toml
  src/tools/Cargo.toml
)

FAILED_TOTAL=0

for manifest in "${MANIFESTS[@]}"; do
  echo "📦 Building $manifest..." | tee -a "$LOG_FILE"
  if cargo build -vv --manifest-path="$manifest" | tee "$TMP_FILE"; then
    echo "✅ Build succeeded for $manifest" | tee -a "$LOG_FILE"
  else
    echo "::error ::Build failed for $manifest!" | tee -a "$LOG_FILE"
    FAILED_TOTAL=$((FAILED_TOTAL + 1))
  fi
done

if [[ "$FAILED_TOTAL" -gt 0 ]]; then
  echo "::error ::Build failed! $FAILED_TOTAL component(s) failed." | tee -a "$LOG_FILE"
  exit 1
fi
