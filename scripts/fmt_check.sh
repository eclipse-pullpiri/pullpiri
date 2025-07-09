#!/bin/bash
set -euo pipefail

LOG_FILE="fmt_results.log"
TMP_FILE="fmt_output.txt"
REPORT_FILE="fmt_summary.md"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo fmt..." | tee -a "$LOG_FILE"

PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

FAILED_TOTAL=0
PASSED_TOTAL=0
PIDS=()

# Declare manifest paths
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"

# Run and parse test output
run_fmt() {
  local manifest="$1"
  local label="$2"
  local fmt_passed=false

  echo "Running fmt for $label ($manifest)" | tee -a "$LOG_FILE"

  if cargo fmt --manifest-path="$manifest" --all --check | tee "$TMP_FILE"; then
    echo "fmt for $label passed clean." | tee -a "$LOG_FILE"
    fmt_passed=true
  else
    echo "::error ::fmt for $label failed! Found warnings/errors. Check logs." | tee -a "$LOG_FILE"
    # Capture relevant lines from TMP_FILE if needed for summary, or direct stdout/stderr
    # Example: Print only the warnings/errors to log, not the whole verbose output
    # grep -E "warning:|error:" "$TMP_FILE" | tee -a "$LOG_FILE"
  fi

  # Instead of PASSED_TOTAL/FAILED_TOTAL for *lints*, we track job success/failure
  if $fmt_passed; then
    echo "✅ fmt for $label: PASSED" >> "$REPORT_FILE"
  else
    echo "❌ fmt for $label: FAILED" >> "$REPORT_FILE"
    # Increment a counter for overall script failure
    (( FAILED_TOTAL++ )) # FAILED_TOTAL now represents number of manifests that failed fmt
  fi
}

# Run common fmt checks
if [[ -f "$COMMON_MANIFEST" ]]; then
  run_fmt "$COMMON_MANIFEST" "common"
else
  echo "::warning ::$COMMON_MANIFEST not found, skipping..."
fi

# Run apiserver fmt checks
if [[ -f "$APISERVER_MANIFEST" ]]; then
  run_fmt "$APISERVER_MANIFEST" "apiserver"
else
  echo "::warning ::$APISERVER_MANIFEST not found, skipping..."
fi

# Run tools fmt checks
if [[ -f "$TOOLS_MANIFEST" ]]; then
  run_fmt "$TOOLS_MANIFEST" "tools"
else
  echo "::warning ::$TOOLS_MANIFEST not found, skipping..."
fi

# Run agent fmt checks
if [[ -f "$AGENT_MANIFEST" ]]; then
  run_fmt "$AGENT_MANIFEST" "agent"
else
  echo "::warning ::$AGENT_MANIFEST not found, skipping..."
fi

# Run filtergateway fmt checks
if [[ -f "$FILTERGATEWAY_MANIFEST" ]]; then
  run_fmt "$FILTERGATEWAY_MANIFEST" "filtergateway"
else
  echo "::warning ::$FILTERGATEWAY_MANIFEST not found, skipping..."
fi

# Run actioncontroller fmt checks
if [[ -f "$ACTIONCONTROLLER_MANIFEST" ]]; then
  run_fmt "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"
else
  echo "::warning ::$ACTIONCONTROLLER_MANIFEST not found, skipping..."
fi