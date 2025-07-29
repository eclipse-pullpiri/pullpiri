#!/bin/bash
set -euo pipefail  # Exit on error, undefined variable, or pipe failure

# Initialize paths and log files
LOG_FILE="clippy_results.log"
TMP_FILE="clippy_output.txt"
mkdir -p dist/reports
REPORT_FILE="dist/reports/clippy_summary.md"

# Clean up old results
rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "🔍 Running Cargo clippy..." | tee -a "$LOG_FILE"

# Set project root (use GitHub workspace if defined, else fallback to current directory)
PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

FAILED_TOTAL=0  # Count how many manifests failed Clippy

# Declare paths to Cargo.toml manifests of components
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"

# Function to run clippy on a component and track results
run_clippy() {
  local manifest="$1"   # Path to Cargo.toml
  local label="$2"      # Human-readable name (e.g., "agent")

  echo "🧪 Running Clippy for $label ($manifest)" | tee -a "$LOG_FILE"

  local clippy_passed=false

  # Run Clippy and capture output to temp file
  if cargo clippy --manifest-path="$manifest" --all-targets --all-features | tee "$TMP_FILE"; then
    echo "✅ Clippy for $label passed clean." | tee -a "$LOG_FILE"
    clippy_passed=true
  else
    echo "::error ::❌ Clippy for $label failed! Found warnings/errors." | tee -a "$LOG_FILE"
    # Optional: uncomment below to include filtered warnings/errors in the log
    # grep -E "warning:|error:" "$TMP_FILE" | tee -a "$LOG_FILE"
  fi

  # Append status to markdown summary report
  if $clippy_passed; then
    echo "✅ Clippy for \`$label\`: **PASSED**" >> "$REPORT_FILE"
  else
    echo "❌ Clippy for \`$label\`: **FAILED**" >> "$REPORT_FILE"
    (( FAILED_TOTAL++ ))  # Increment failure count
  fi
}

# === Clippy runs per module ===

[[ -f "$COMMON_MANIFEST" ]] && run_clippy "$COMMON_MANIFEST" "common" \
  || echo "::warning ::$COMMON_MANIFEST not found, skipping..."

[[ -f "$APISERVER_MANIFEST" ]] && run_clippy "$APISERVER_MANIFEST" "apiserver" \
  || echo "::warning ::$APISERVER_MANIFEST not found, skipping..."

[[ -f "$TOOLS_MANIFEST" ]] && run_clippy "$TOOLS_MANIFEST" "tools" \
  || echo "::warning ::$TOOLS_MANIFEST not found, skipping..."

[[ -f "$AGENT_MANIFEST" ]] && run_clippy "$AGENT_MANIFEST" "agent" \
  || echo "::warning ::$AGENT_MANIFEST not found, skipping..."

[[ -f "$FILTERGATEWAY_MANIFEST" ]] && run_clippy "$FILTERGATEWAY_MANIFEST" "filtergateway" \
  || echo "::warning ::$FILTERGATEWAY_MANIFEST not found, skipping..."

[[ -f "$ACTIONCONTROLLER_MANIFEST" ]] && run_clippy "$ACTIONCONTROLLER_MANIFEST" "actioncontroller" \
  || echo "::warning ::$ACTIONCONTROLLER_MANIFEST not found, skipping..."

# Optional: exit with failure if any component had Clippy errors
if [[ "$FAILED_TOTAL" -gt 0 ]]; then
  echo "::error ::🚨 Clippy failed for $FAILED_TOTAL component(s)." | tee -a "$LOG_FILE"
  exit 1
fi
