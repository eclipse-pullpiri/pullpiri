#!/bin/bash
set -euo pipefail

PROJECT_ROOT="$(pwd)"
mkdir -p "$PROJECT_ROOT/dist/licenses"
LOG_FILE="$PROJECT_ROOT/dist/licenses/license_log.txt"
rm -f "$LOG_FILE"
touch "$LOG_FILE"

echo "🔍 Starting license checks..." | tee -a "$LOG_FILE"

MANIFESTS=(
    "src/server/Cargo.toml"
    "src/common/Cargo.toml"
    "src/agent/Cargo.toml"
    "src/tools/Cargo.toml"
    "src/player/Cargo.toml"
)

# Ensure cargo-about is installed
if ! command -v cargo-about &>/dev/null; then
  echo "❗ cargo-about not found, installing..." | tee -a "$LOG_FILE"
  cargo install cargo-about
fi

for manifest in "${MANIFESTS[@]}"; do
  if [[ -f "$manifest" ]]; then
    crate_dir="$(dirname "$manifest")"
    label="$(basename "$crate_dir")"

    CONFIG="$PROJECT_ROOT/$crate_dir/about.toml"
    TEMPLATE="$PROJECT_ROOT/$crate_dir/about.hbs"

    if [[ ! -f "$CONFIG" ]]; then
      echo "::error ::Missing $CONFIG for $label. Skipping..." | tee -a "$LOG_FILE"
      continue
    fi
    if [[ ! -f "$TEMPLATE" ]]; then
      echo "::error ::Missing $TEMPLATE for $label. Skipping..." | tee -a "$LOG_FILE"
      continue
    fi

    echo "📄 Generating license report for $label ($manifest)" | tee -a "$LOG_FILE"
    echo "Using template: $TEMPLATE" | tee -a "$LOG_FILE"
    echo "Using config: $CONFIG" | tee -a "$LOG_FILE"

    output_path="$PROJECT_ROOT/dist/licenses/${label}_licenses.html"
    mkdir -p "$(dirname "$output_path")"

    (
      cd "$crate_dir"
      echo "🔧 Working in $(pwd), generating $output_path" | tee -a "$LOG_FILE"
      cargo about generate --config "$CONFIG" "$TEMPLATE" > "$output_path"
    )
  else
    echo "::warning ::Manifest $manifest not found, skipping..." | tee -a "$LOG_FILE"
  fi
done

echo "✅ License reports generated in dist/licenses" | tee -a "$LOG_FILE"
