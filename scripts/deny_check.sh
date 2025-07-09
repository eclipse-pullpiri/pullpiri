#!/bin/bash
set -euo pipefail

LOG_FILE="deny_results.log"
TMP_FILE="deny_output.txt"
REPORT_FILE="deny_summary.md"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "🔍 Running Cargo Deny checks..." | tee -a "$LOG_FILE"

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

# Function to run cargo-deny check
run_deny() {
  local manifest="$1"
  local label="$2"
  local deny_passed=false

  echo "🚨 Running deny check for $label ($manifest)" | tee -a "$LOG_FILE"

  if cargo deny --manifest-path="$manifest" check 2>&1 | tee "$TMP_FILE"; then
    echo "✅ deny check for $label passed clean." | tee -a "$LOG_FILE"
    deny_passed=true
  else
    echo "::error ::Deny check for $label failed! Issues found." | tee -a "$LOG_FILE"
    # Optional: extract relevant issues
    grep -E "error:|warning:" "$TMP_FILE" | tee -a "$LOG_FILE"
  fi

  if $deny_passed; then
    echo "✅ deny check for $label: PASSED" >> "$REPORT_FILE"
    (( PASSED_TOTAL++ ))
  else
    echo "❌ deny check for $label: FAILED" >> "$REPORT_FILE"
    (( FAILED_TOTAL++ ))
  fi
}

# Run deny check for each manifest
#[[ -f "$COMMON_MANIFEST" ]]       && run_deny "$COMMON_MANIFEST" "common"        || echo "::warning ::$COMMON_MANIFEST not found, skipping..."
#[[ -f "$AGENT_MANIFEST" ]]        && run_deny "$AGENT_MANIFEST" "agent"          || echo "::warning ::$AGENT_MANIFEST not found, skipping..."
#[[ -f "$TOOLS_MANIFEST" ]]        && run_deny "$TOOLS_MANIFEST" "tools"          || echo "::warning ::$TOOLS_MANIFEST not found, skipping..."
[[ -f "$APISERVER_MANIFEST" ]]    && run_deny "$APISERVER_MANIFEST" "apiserver"  || echo "::warning ::$APISERVER_MANIFEST not found, skipping..."
#[[ -f "$FILTERGATEWAY_MANIFEST" ]]&& run_deny "$FILTERGATEWAY_MANIFEST" "filtergateway" || echo "::warning ::$FILTERGATEWAY_MANIFEST not found, skipping..."

# Final summary
echo -e "\n📄 Summary:" | tee -a "$LOG_FILE"
cat "$REPORT_FILE" | tee -a "$LOG_FILE"

echo -e "\n🔢 Total Passed: $PASSED_TOTAL" | tee -a "$LOG_FILE"
echo "🔢 Total Failed: $FAILED_TOTAL" | tee -a "$LOG_FILE"

# Fail script if any deny check failed
if [[ "$FAILED_TOTAL" -gt 0 ]]; then
  echo "::error ::One or more cargo-deny checks failed."
  exit 1
fi

echo "✅ All cargo-deny checks passed!"