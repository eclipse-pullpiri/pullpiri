#!/bin/bash
set -euo pipefail

# === Initialize paths and variables ===
LOG_FILE="test_results.log"
TMP_FILE="test_output.txt"
mkdir -p dist/tests target
REPORT_FILE="dist/tests/test_summary.xml"

# Clean up any previous log or report files
rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "🚀 Running Cargo Tests..." | tee -a "$LOG_FILE"

# Detect project root (for CI or local)
PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

# Track total test stats
FAILED_TOTAL=0
PASSED_TOTAL=0
PIDS=()  # List of background service PIDs to kill later

# === Path to Cargo.toml for each Rust subproject ===
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"
STATEMANAGER_MANIFEST="src/player/statemanager/Cargo.toml"

# === Function: Start background service ===
start_service() {
  local manifest="$1"
  local name="$2"
  echo "🔄 Starting $name..." | tee -a "$LOG_FILE"
  cargo run --manifest-path="$manifest" &>> "$LOG_FILE" &  # Run in background
  PIDS+=($!)  # Track PID
}

# === Function: Stop all background services ===
cleanup() {
  echo -e "\n🧹 Stopping services..." | tee -a "$LOG_FILE"
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" &>/dev/null; then
      kill "$pid" 2>/dev/null || echo "⚠️ Could not kill $pid"
    fi
  done
  PIDS=()  # Reset PID list
}
trap cleanup EXIT  # Ensure cleanup is called on exit

# === Function: Run tests for a given manifest ===
run_tests() {
  local manifest="$1"
  local label="$2"
  local output_json="target/${label}_test_output.json"
  local report_xml="dist/tests/${label}_results.xml"

  echo "🧪 Testing $label ($manifest)" | tee -a "$LOG_FILE"

  # Run tests and capture structured JSON output
  if RUSTC_BOOTSTRAP=1 cargo test --manifest-path="$manifest" -- -Z unstable-options --format json > "$output_json" 2>>"$LOG_FILE"; then
    echo "✅ Tests passed for $label" | tee -a "$LOG_FILE"
  else
    echo "::error ::❌ Tests failed for $label!" | tee -a "$LOG_FILE"
  fi

  # === Parse test output ===
  if [[ -f "$output_json" ]]; then
    if command -v jq &>/dev/null; then
      # Count number of passed and failed tests using jq
      passed=$(jq -r 'select(.type == "test" and .event == "ok") | .name' "$output_json" | wc -l)
      failed=$(jq -r 'select(.type == "test" and .event == "failed") | .name' "$output_json" | wc -l)
    else
      echo "::warning ::jq not found, cannot parse JSON test output."
      passed=0
      failed=0
    fi

    PASSED_TOTAL=$((PASSED_TOTAL + passed))
    FAILED_TOTAL=$((FAILED_TOTAL + failed))

    echo "ℹ️ Passed: $passed, Failed: $failed" | tee -a "$LOG_FILE"

    # Convert JSON to JUnit-style XML if cargo2junit is installed
    if command -v cargo2junit &>/dev/null; then
      cargo2junit < "$output_json" > "$report_xml"
    else
      echo "::warning ::cargo2junit not found, skipping XML for $label"
    fi
  else
    echo "::warning ::No output file $output_json created for $label" | tee -a "$LOG_FILE"
  fi
}

# === Start IDL2DDS Docker Service if not already running ===
if ! docker ps | grep -qi "idl2dds"; then
  echo "📦 Launching IDL2DDS docker services..." | tee -a "$LOG_FILE"
  [[ ! -d IDL2DDS ]] && git clone https://github.com/MCO-PICCOLO/IDL2DDS -b master

  pushd IDL2DDS
  docker compose up --build -d
  popd
else
  echo "🟢 IDL2DDS already running." | tee -a "$LOG_FILE"
fi

# === Run tests for each Rust component ===

# Step 1: Run tests for `common`
[[ -f "$COMMON_MANIFEST" ]] && run_tests "$COMMON_MANIFEST" "common" || echo "::warning ::$COMMON_MANIFEST missing."

# Step 2: Start `filtergateway` and `nodeagent` before testing `apiserver`
start_service "$FILTERGATEWAY_MANIFEST" "filtergateway"
start_service "$AGENT_MANIFEST" "nodeagent"
sleep 3  # Give services time to initialize
start_service "$STATEMANAGER_MANIFEST" "statemanager"
[[ -f "$APISERVER_MANIFEST" ]] && run_tests "$APISERVER_MANIFEST" "apiserver" || echo "::warning ::$APISERVER_MANIFEST missing."
cleanup  # Stop background services

# Step 3: Test `tools` (and optionally `agent`)
[[ -f "$TOOLS_MANIFEST" ]] && run_tests "$TOOLS_MANIFEST" "tools" || echo "::warning ::$TOOLS_MANIFEST missing."
# [[ -f "$AGENT_MANIFEST" ]] && run_tests "$AGENT_MANIFEST" "agent" || echo "::warning ::$AGENT_MANIFEST missing."

# Step 4: Start `actioncontroller` and `statemanager` before testing `filtergateway`
start_service "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"
start_service "$STATEMANAGER_MANIFEST" "statemanager"
etcdctl del "" --prefix
sleep 3
[[ -f "$FILTERGATEWAY_MANIFEST" ]] && run_tests "$FILTERGATEWAY_MANIFEST" "filtergateway" || echo "::warning ::$FILTERGATEWAY_MANIFEST missing."
cleanup  # Stop actioncontroller

# Step 5: Test `statemanager`
[[ -f "$STATEMANAGER_MANIFEST" ]] && run_tests "$STATEMANAGER_MANIFEST" "statemanager" || echo "::warning ::$STATEMANAGER_MANIFEST missing."

# Optional: Uncomment to test `actioncontroller` directly
# [[ -f "$ACTIONCONTROLLER_MANIFEST" ]] && run_tests "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"

# === Combine all JUnit test reports into one XML file ===
echo "<?xml version=\"1.0\" encoding=\"UTF-8\"?>" > "$REPORT_FILE"
echo "<testsuites>" >> "$REPORT_FILE"
for xml in dist/tests/*_results.xml; do
  [[ -f "$xml" ]] && cat "$xml" >> "$REPORT_FILE"
done
echo "</testsuites>" >> "$REPORT_FILE"

# === Final test summary ===
echo "✅ Tests Passed: $PASSED_TOTAL" | tee -a "$LOG_FILE"
echo "❌ Tests Failed: $FAILED_TOTAL" | tee -a "$LOG_FILE"

# If any test failed, exit with error so CI fails
[[ "$FAILED_TOTAL" -gt 0 ]] && {
  echo "::error ::Some tests failed!" | tee -a "$LOG_FILE"
  exit 1
}

echo "🎉 All tests passed!" | tee -a "$LOG_FILE"
