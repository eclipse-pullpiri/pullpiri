#!/bin/bash

# Exit immediately on error, undefined variable, or pipeline failure
set -euo pipefail

# Set up log files
LOG_FILE="build_results.log"
TMP_FILE="build_output.txt"
rm -f "$LOG_FILE" "$TMP_FILE"

echo "Running Cargo Build..." | tee -a "$LOG_FILE"

# Determine the root of the Git project or use current directory
PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")

# Mark the project directory as safe for Git (important in CI to avoid "unsafe repository" warnings)
git config --global --add safe.directory "$PROJECT_ROOT"

# Navigate to the project root
cd "$PROJECT_ROOT"

# List of Cargo manifest paths for different components
MANIFESTS=(
  src/common/Cargo.toml
  src/agent/Cargo.toml
  src/player/Cargo.toml
  src/server/Cargo.toml
  src/tools/Cargo.toml
)

FAILED_TOTAL=0  # Track how many builds failed

# Loop through each manifest and try to build
for manifest in "${MANIFESTS[@]}"; do
  echo "📦 Building $manifest..." | tee -a "$LOG_FILE"

  # Build with verbose output; capture to temp file and tee to log
  if cargo build -vv --manifest-path="$manifest" | tee "$TMP_FILE"; then
    echo "✅ Build succeeded for $manifest" | tee -a "$LOG_FILE"
  else
    echo "::error ::❌ Build failed for $manifest!" | tee -a "$LOG_FILE"
    FAILED_TOTAL=$((FAILED_TOTAL + 1))
  fi
done

# Final result: fail the script if any build failed
if [[ "$FAILED_TOTAL" -gt 0 ]]; then
  echo "::error ::🚨 Build failed! $FAILED_TOTAL component(s) failed." | tee -a "$LOG_FILE"
  exit 1
fi
