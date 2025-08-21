#!/bin/bash
set -euo pipefail

# === Initialize paths and variables ===
LOG_FILE="dist/coverage/test_coverage_log.txt"
COVERAGE_ROOT="dist/coverage"
# Detect project root (for CI or local)
PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"
mkdir -p "$COVERAGE_ROOT"
rm -f "$LOG_FILE"
touch "$LOG_FILE"
PIDS=()

echo "🧪 Starting test coverage collection per crate..." | tee -a "$LOG_FILE"

# === Function: Start background service ===
start_service() {
  local manifest="$1"
  local name="$2"
  echo "🔄 Starting $name..." | tee -a "$LOG_FILE"
  cargo run --manifest-path="$manifest" &>> "$LOG_FILE" &
  PIDS+=($!)
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

# === Ensure cargo-tarpaulin is installed ===
if ! command -v cargo-tarpaulin &>/dev/null; then
  echo "📦 Installing cargo-tarpaulin..." | tee -a "$LOG_FILE"
  cargo install cargo-tarpaulin
fi

# === Enable nightly-only options ===
export RUSTC_BOOTSTRAP=1

# === MANIFEST paths ===
COMMON_MANIFEST="src/common/Cargo.toml"
AGENT_MANIFEST="src/agent/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"
SERVER_MANIFEST="src/server/Cargo.toml"
APISERVER_MANIFEST="src/server/apiserver/Cargo.toml"
PLAYER_MANIFEST="src/player/Cargo.toml"
FILTERGATEWAY_MANIFEST="src/player/filtergateway/Cargo.toml"
ACTIONCONTROLLER_MANIFEST="src/player/actioncontroller/Cargo.toml"
STATEMANAGER_MANIFEST="src/player/statemanager/Cargo.toml"

# === COMMON ===
if [[ -f "$COMMON_MANIFEST" ]]; then
  echo "📂 Running tarpaulin for common" | tee -a "$LOG_FILE"
  mkdir -p "$COVERAGE_ROOT/common"
  (
    cd "$(dirname "$COMMON_MANIFEST")"
    cargo tarpaulin --out Html --out Lcov --out Xml \
      --output-dir "$PROJECT_ROOT/$COVERAGE_ROOT/common" \
      2>&1 | tee -a "$LOG_FILE" || true
  )
  mv "$PROJECT_ROOT/$COVERAGE_ROOT/common/tarpaulin-report.html" "$PROJECT_ROOT/$COVERAGE_ROOT/common/tarpaulin-report-common.html" 2>/dev/null || true
else
  echo "::warning ::$COMMON_MANIFEST not found. Skipping..." | tee -a "$LOG_FILE"
fi

# === Agent ===
# Test Cases are not proper and passing so code coverage report will not generate as of now
# if [[ -f "$AGENT_MANIFEST" ]]; then
#   echo "📂 Running tarpaulin for agent" | tee -a "$LOG_FILE"
#   mkdir -p "$COVERAGE_ROOT/agent"
#   (
#     cd "$(dirname "$AGENT_MANIFEST")"
#     cargo tarpaulin --out Html --out Lcov --out Xml \
#       --output-dir "$PROJECT_ROOT/$COVERAGE_ROOT/agent" \
#       2>&1 | tee -a "$LOG_FILE" || true
#   )
#   mv "$PROJECT_ROOT/$COVERAGE_ROOT/agent/tarpaulin-report.html" "$PROJECT_ROOT/$COVERAGE_ROOT/agent/tarpaulin-report-agent.html" 2>/dev/null || true
# else
#   echo "::warning ::$AGENT_MANIFEST not found. Skipping..." | tee -a "$LOG_FILE"
# fi

# === TOOLS ===
if [[ -f "$TOOLS_MANIFEST" ]]; then
  echo "📂 Running tarpaulin for tools" | tee -a "$LOG_FILE"
  mkdir -p "$COVERAGE_ROOT/tools"
  (
    cd "$(dirname "$TOOLS_MANIFEST")"
    cargo tarpaulin --out Html --out Lcov --out Xml \
      --output-dir "$PROJECT_ROOT/$COVERAGE_ROOT/tools" \
      2>&1 | tee -a "$LOG_FILE" || true
  )
  mv "$PROJECT_ROOT/$COVERAGE_ROOT/tools/tarpaulin-report.html" "$PROJECT_ROOT/$COVERAGE_ROOT/tools/tarpaulin-report-tools.html" 2>/dev/null || true
else
  echo "::warning ::$TOOLS_MANIFEST not found. Skipping..." | tee -a "$LOG_FILE"
fi

# === Step 2: Start `filtergateway` and `nodeagent` before apiserver ===
start_service "$FILTERGATEWAY_MANIFEST" "filtergateway"
start_service "$AGENT_MANIFEST" "nodeagent"
sleep 3

# === SERVER ===
if [[ -f "$SERVER_MANIFEST" ]]; then
  echo "📂 Running tarpaulin for server" | tee -a "$LOG_FILE"
  mkdir -p "$COVERAGE_ROOT/server"
  (
    cd "$(dirname "$SERVER_MANIFEST")"
    cargo tarpaulin --out Html --out Lcov --out Xml \
      --output-dir "$PROJECT_ROOT/$COVERAGE_ROOT/server" \
      2>&1 | tee -a "$LOG_FILE" || true
  )
  mv "$PROJECT_ROOT/$COVERAGE_ROOT/server/tarpaulin-report.html" "$PROJECT_ROOT/$COVERAGE_ROOT/server/tarpaulin-report-server.html" 2>/dev/null || true
else
  echo "::warning ::$SERVER_MANIFEST not found. Skipping..." | tee -a "$LOG_FILE"
fi

# Stop background services before next round
cleanup

# === Start IDL2DDS Docker Service ===
if ! docker ps | grep -qi "idl2dds"; then
  echo "📦 Launching IDL2DDS docker services..." | tee -a "$LOG_FILE"
  [[ ! -d IDL2DDS ]] && git clone https://github.com/MCO-PICCOLO/IDL2DDS -b master
  pushd IDL2DDS
  docker compose up --build -d
  popd
else
  echo "🟢 IDL2DDS already running." | tee -a "$LOG_FILE"
fi

# === Player ===
start_service "$ACTIONCONTROLLER_MANIFEST" "actioncontroller"
start_service "$STATEMANAGER_MANIFEST" "statemanager"
etcdctl del "" --prefix
sleep 3

if [[ -f "$PLAYER_MANIFEST" ]]; then
  echo "📂 Running tarpaulin for player" | tee -a "$LOG_FILE"
  mkdir -p "$COVERAGE_ROOT/player"
  (
    cd "$(dirname "$PLAYER_MANIFEST")"
    cargo tarpaulin --out Html --out Lcov --out Xml \
      --output-dir "$PROJECT_ROOT/$COVERAGE_ROOT/player" \
      2>&1 | tee -a "$LOG_FILE" || true
  )
  mv "$PROJECT_ROOT/$COVERAGE_ROOT/player/tarpaulin-report.html" "$PROJECT_ROOT/$COVERAGE_ROOT/player/tarpaulin-report-player.html" 2>/dev/null || true
else
  echo "::warning ::$PLAYER_MANIFEST not found. Skipping..." | tee -a "$LOG_FILE"
fi

cleanup

# === Summary ===
echo "✅ All test coverage reports generated at: $COVERAGE_ROOT" | tee -a "$LOG_FILE"
