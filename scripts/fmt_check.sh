#!/bin/bash
set -euo pipefail  # Exit immediately on error, unset variable, or pipe failure

LOG_FILE="fmt_results.log"
TMP_FILE="fmt_output.txt"

# Create directory to store formatting reports
mkdir -p dist/reports/fmt
REPORT_FILE="dist/reports/fmt/fmt_summary.md"

# Clean up old logs and reports before starting
rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo fmt..." | tee -a "$LOG_FILE"

# Determine project root: use GitHub workspace if set, else current dir
PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

FAILED_TOTAL=0  # Counter for failed formatting checks
PASSED_TOTAL=0  # Counter for passed formatting checks
PIDS=()        # (Unused here but declared in case of future parallel runs)

# Declare paths to Cargo.toml manifests for different crates/components
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"

# Function to run 'cargo fmt --check' on a given manifest and record results
run_fmt() {
  local manifest="$1"
  local label="$2"
  local fmt_passed=false

  echo "Running fmt for $label ($manifest)" | tee -a "$LOG_FILE"

  # Run cargo fmt in check mode; output to TMP_FILE
  if cargo fmt --manifest-path="$manifest" --all --check | tee "$TMP_FILE"; then
    echo "fmt for $label passed clean." | tee -a "$LOG_FILE"
    fmt_passed=true
  else
    echo "::error ::fmt for $label failed! Found formatting issues. Check logs." | tee -a "$LOG_FILE"
    # Optionally, you could extract and log only relevant warnings/errors from TMP_FILE
    # e.g., grep -E "warning:|error:" "$TMP_FILE" | tee -a "$LOG_FILE"
  fi

  # Append pass/fail status to markdown summary report
  if $fmt_passed; then
    echo "✅ fmt for $label: PASSED" >> "$REPORT_FILE"
  else
    echo "❌ fmt for $label: FAILED" >> "$REPORT_FILE"
    (( FAILED_TOTAL++ ))  # Increment failure count
  fi
}

# Run formatting checks for each crate manifest if the file exists
if [[ -f "$COMMON_MANIFEST" ]]; then
  run_fmt "$COMMON_MANIFEST" "common"
else
  echo "::warning ::$COMMON_MANIFEST not found, skipping..."
fi

if [[ -f "$APISERVER_MANIFEST" ]]; then
  run_fmt "$APISERVER_MANIFEST" "apiserver"
else
  echo "::warning ::$APISERVER_MANIFEST not found, skipping..."
fi

if [[ -f "$TOOLS_MANIFEST" ]]; then
  run_fmt "$TOOLS_MANIFEST" "tools"
else
  echo "::warning ::$TOOLS_MANIFEST not found, skipping..."
fi

if [[ -f "$AGENT_MANIFEST" ]]; then
  run_fmt "$AGENT_MANIFEST" "agent"
else
  echo "::warning ::$AGENT_MANIFEST not found, skipping..."
fi

if [[ -f "$FILTERGATEWAY_MANIFEST" ]]; then
  run_fmt "$FILTERGATEWAY_MANIFEST" "filtergateway"
else
  echo "::warning ::$FILTERGATEWAY_MANIFEST not found, skipping..."
fi

if [[ -f "$ACTIONCONTROLLER_MANIFEST" ]]; then
  run_fmt "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"
else
  echo "::warning ::$ACTIONCONTROLLER_MANIFEST not found, skipping..."
fi
