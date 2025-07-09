#!/bin/bash
set -euo pipefail

LOG_FILE="test_results.log"
TMP_FILE="test_output.txt"
REPORT_FILE="test_summary.md"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo Tests..." | tee -a "$LOG_FILE"

PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

FAILED_TOTAL=0
PASSED_TOTAL=0
PIDS=()

# Declare manifest pathss
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"

# Start background service and save its PID
start_service() {
  local manifest="$1"
  local name="$2"

  echo "Starting $name component for testing..." | tee -a "$LOG_FILE"
  cargo run --manifest-path="$manifest" &>> "$LOG_FILE" &
  PIDS+=($!)
}

# Ensure background processes are cleaned up
cleanup() {
  echo -e "\n Cleaning up background services..." | tee -a "$LOG_FILE"
  kill "${PIDS[@]}" 2>/dev/null || true
}
trap cleanup EXIT

# Run and parse test output
run_tests() {
  local manifest="$1"
  local label="$2"

  echo "Running tests for $label ($manifest)" | tee -a "$LOG_FILE"

  if cargo test -vv --manifest-path="$manifest" -- --test-threads=1 | tee "$TMP_FILE"; then
    echo "Tests passed for $label"
  else
    echo "::error ::Tests failed for $label! Check logs." | tee -a "$LOG_FILE"
  fi

  local passed
  local failed

  passed=$(grep -oP '\d+ passed' "$TMP_FILE" | awk '{sum += $1} END {print sum}')
  failed=$(grep -oP '\d+ failed' "$TMP_FILE" | awk '{sum += $1} END {print sum}')

  PASSED_TOTAL=$((PASSED_TOTAL + passed))
  FAILED_TOTAL=$((FAILED_TOTAL + failed))
}

# Run common tests
if [[ -f "$COMMON_MANIFEST" ]]; then
  run_tests "$COMMON_MANIFEST" "common"
else
  echo "::warning ::$COMMON_MANIFEST not found, skipping..."
fi

# Start services required for apiserver
start_service "$FILTERGATEWAY_MANIFEST" "filtergateway"
start_service "$AGENT_MANIFEST" "nodeagent"

# Wait for services to be ready (simple delay)
sleep 3

# Run apiserver tests
if [[ -f "$APISERVER_MANIFEST" ]]; then
  run_tests "$APISERVER_MANIFEST" "apiserver"
else
  echo "::warning ::$APISERVER_MANIFEST not found, skipping..."
fi

# Stop only those services needed for apiserver
cleanup

# Re-setup trap for any new background processes started later
PIDS=()
trap cleanup EXIT

# Run tools tests
if [[ -f "$TOOLS_MANIFEST" ]]; then
  run_tests "$TOOLS_MANIFEST" "tools"
else
  echo "::warning ::$TOOLS_MANIFEST not found, skipping..."
fi

# Run agent tests
if [[ -f "$AGENT_MANIFEST" ]]; then
  run_tests "$AGENT_MANIFEST" "agent"
else
  echo "::warning ::$AGENT_MANIFEST not found, skipping..."
fi

# Run filtergateway tests(development is under progress)
# if [[ -f "$FILTERGATEWAY_MANIFEST" ]]; then
#   run_tests "$FILTERGATEWAY_MANIFEST" "filtergateway"
# else
#   echo "::warning ::$FILTERGATEWAY_MANIFEST not found, skipping..."
# fi

# Run actioncontroller tests(development is under progress)
# if [[ -f "$ACTIONCONTROLLER_MANIFEST" ]]; then
#   run_tests "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"
# else
#   echo "::warning ::$ACTIONCONTROLLER_MANIFEST not found, skipping..."
# fi

# Generate test summary report
echo "# Test Results" > "$REPORT_FILE"
echo "**Passed:** $PASSED_TOTAL" >> "$REPORT_FILE"
echo "**Failed:** $FAILED_TOTAL" >> "$REPORT_FILE"

echo "Tests Passed: $PASSED_TOTAL" | tee -a "$LOG_FILE"
echo "Tests Failed: $FAILED_TOTAL" | tee -a "$LOG_FILE"

if [[ "$FAILED_TOTAL" -gt 0 ]]; then
  echo "::error ::Some tests failed!" | tee -a "$LOG_FILE"
  exit 1
fi

echo "All tests passed successfully!" | tee -a "$LOG_FILE"
