#!/usr/bin/env bash
#
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
#
# Dynamic Resource Scaling - full-stack end-to-end test (#514 / #526)
#
# Brings up the real pullpiri components involved in the scaling path and
# drives a scaling request through them, exactly as production would:
#
#   scaling_client --RequestResourceScaling--> ActionController (47001, container)
#        -> ResourceManager   (47008, container)   validate / desired / reconcile
#        -> NodeAgent         (47004, host binary)  Podman libpod update
#        -> target container cgroup (cpu.max / memory.max)   <-- authoritative check
#
# Requirements:
#   - Container image `localhost/pullpiri:latest`  (build with: make image)
#   - podman available (>= 4.3). The Podman API socket is auto-detected and,
#     if absent, a transient `podman system service` is started/stopped.
#     Works for both root (rootful) and rootless Podman.
#   - Rust toolchain (cargo) to build the NodeAgent binary + example client.
#
# Usage:
#   sudo scripts/e2e_dynamic_resource_scaling.sh     # root (rootful) podman
#   scripts/e2e_dynamic_resource_scaling.sh          # rootless podman
#   IMAGE=docker.io/library/alpine scripts/e2e_dynamic_resource_scaling.sh

set -euo pipefail

# --- Make `cargo` reachable, even under sudo -------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  for _c in \
    "${SUDO_USER:+/home/$SUDO_USER/.cargo/bin}" \
    "${HOME:+$HOME/.cargo/bin}" \
    "/root/.cargo/bin"; do
    if [ -n "$_c" ] && [ -x "$_c/cargo" ]; then
      export PATH="$_c:$PATH"
      _home="$(dirname "$(dirname "$_c")")"
      export RUSTUP_HOME="${RUSTUP_HOME:-$_home/.rustup}"
      export CARGO_HOME="${CARGO_HOME:-$_home/.cargo}"
      break
    fi
  done
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: 'cargo' not found. Install Rust or run: sudo -E env \"PATH=\$PATH\" $0" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/src"
NODEAGENT="$SRC/agent/nodeagent"
IMAGE="${IMAGE:-docker.io/library/busybox}"
CONTAINER="${CONTAINER:-pullpiri-scaling-test}"
PP_IMAGE="${PP_IMAGE:-localhost/pullpiri:latest}"
WORKDIR="$(mktemp -d /tmp/pullpiri-e2e.XXXXXX)"
NA_LOG="$WORKDIR/nodeagent.log"

echo "======================================================================"
echo " Dynamic Resource Scaling - full-stack E2E (#514 / #526)"
echo "======================================================================"

# --- Preconditions ----------------------------------------------------------
if ! command -v podman >/dev/null 2>&1 || ! podman info >/dev/null 2>&1; then
  echo "ERROR: podman not available or not usable." >&2
  exit 1
fi
if ! podman image exists "$PP_IMAGE"; then
  echo "ERROR: image '$PP_IMAGE' not found. Build it first with: make image" >&2
  exit 1
fi

# --- Resolve the Podman API socket -----------------------------------------
if [ -z "${PODMAN_SOCKET:-}" ]; then
  PODMAN_SOCKET="$(podman info --format '{{.Host.RemoteSocket.Path}}' 2>/dev/null || true)"
  [ -z "$PODMAN_SOCKET" ] && PODMAN_SOCKET="/run/podman/podman.sock"
fi
export PODMAN_SOCKET
echo "-- Podman socket: $PODMAN_SOCKET"

SVC_PID=""; NA_PID=""
if [ ! -S "$PODMAN_SOCKET" ]; then
  echo "-- socket not found, starting a transient Podman API service"
  mkdir -p "$(dirname "$PODMAN_SOCKET")"
  podman system service --time=0 "unix://$PODMAN_SOCKET" \
    >"$WORKDIR/podman-svc.log" 2>&1 &
  SVC_PID=$!
  for _ in $(seq 1 20); do [ -S "$PODMAN_SOCKET" ] && break; sleep 0.5; done
  [ -S "$PODMAN_SOCKET" ] || { echo "failed to start Podman API service"; exit 1; }
fi

cleanup() {
  echo
  echo "-- cleanup"
  [ -n "$NA_PID" ] && kill "$NA_PID" >/dev/null 2>&1 || true
  podman rm -f "$CONTAINER" pullpiri-e2e-actioncontroller pullpiri-e2e-resourcemanager \
    >/dev/null 2>&1 || true
  [ -n "$SVC_PID" ] && kill "$SVC_PID" >/dev/null 2>&1 || true
  rm -rf "$WORKDIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# --- Config for the pullpiri services --------------------------------------
# ip 0.0.0.0 => servers bind on all interfaces; clients dial 127.0.0.1.
cat > "$WORKDIR/settings.yaml" <<'EOF'
host:
  name: HPC
  ip: 0.0.0.0
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
EOF

cat > "$WORKDIR/nodeagent.yaml" <<'EOF'
nodeagent:
  node_name: "HPC"
  node_type: "vehicle"
  node_role: "nodeagent"
  master_ip: "127.0.0.1"
  node_ip: "0.0.0.0"
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "HPC"
    platform: "Linux"
    architecture: "x86_64"
EOF

# --- Build the NodeAgent binary + example client ----------------------------
echo
echo "== Build ============================================================"
echo "-- NodeAgent binary (release)"
( cd "$NODEAGENT" && cargo build --release >/dev/null )
NA_BIN="$NODEAGENT/target/release/nodeagent"
echo "-- scaling_client example"
( cd "$SRC" && cargo build -p actioncontroller --example scaling_client >/dev/null )

wait_port() { # host port name
  for _ in $(seq 1 40); do
    (exec 3<>"/dev/tcp/$1/$2") 2>/dev/null && { exec 3>&- 3<&-; return 0; }
    sleep 0.25
  done
  echo "ERROR: $3 did not start on $1:$2" >&2; return 1
}

# --- Start the server-side pullpiri components ------------------------------
echo
echo "== Start components ================================================="
echo "-- ResourceManager (container, 47008)"
podman rm -f pullpiri-e2e-resourcemanager >/dev/null 2>&1 || true
podman run -d --name pullpiri-e2e-resourcemanager --network host \
  -v "$WORKDIR/settings.yaml:/etc/pullpiri/settings.yaml:Z" \
  "$PP_IMAGE" /pullpiri/resourcemanager >/dev/null
wait_port 127.0.0.1 47008 "ResourceManager"

echo "-- ActionController (container, 47001)"
podman rm -f pullpiri-e2e-actioncontroller >/dev/null 2>&1 || true
podman run -d --name pullpiri-e2e-actioncontroller --network host \
  -v "$WORKDIR/settings.yaml:/etc/pullpiri/settings.yaml:Z" \
  "$PP_IMAGE" /pullpiri/actioncontroller >/dev/null
wait_port 127.0.0.1 47001 "ActionController"

echo "-- NodeAgent (host binary, 47004)"
PODMAN_SOCKET="$PODMAN_SOCKET" "$NA_BIN" --config "$WORKDIR/nodeagent.yaml" \
  >"$NA_LOG" 2>&1 &
NA_PID=$!
wait_port 127.0.0.1 47004 "NodeAgent"

# --- Create the workload container to be scaled -----------------------------
echo
echo "== Scenario ========================================================="
echo "-- creating workload container '$CONTAINER' (2 cores / 512 MiB)"
podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
podman run -d --name "$CONTAINER" --cpus 2 --memory 512m "$IMAGE" sleep 600 >/dev/null
echo "   BEFORE: $(podman inspect "$CONTAINER" \
  --format 'NanoCpus={{.HostConfig.NanoCpus}} Memory={{.HostConfig.Memory}}')"

# --- Drive the scaling request through the whole stack ----------------------
echo
echo "-- RequestResourceScaling -> scale DOWN to 1 core / 256 MiB"
( cd "$SRC" && cargo run -q -p actioncontroller --example scaling_client -- \
    "$CONTAINER" 1 256 down 127.0.0.1 )

# --- Verify against the real cgroup (authoritative) -------------------------
echo
echo "== Verify (cgroup is authoritative) ================================="
CID="$(podman inspect "$CONTAINER" --format '{{.Id}}')"
read_cgroup() { # metric-file
  podman exec "$CONTAINER" cat "/sys/fs/cgroup/$1" 2>/dev/null || \
    cat "/sys/fs/cgroup/$(dirname "$(grep -m1 . /proc/self/cgroup)")" 2>/dev/null || true
}
CPU_MAX="$(podman exec "$CONTAINER" cat /sys/fs/cgroup/cpu.max 2>/dev/null || echo '?')"
MEM_MAX="$(podman exec "$CONTAINER" cat /sys/fs/cgroup/memory.max 2>/dev/null || echo '?')"
echo "   cpu.max    = $CPU_MAX     (expect: 100000 100000  -> 1 core)"
echo "   memory.max = $MEM_MAX   (expect: 268435456      -> 256 MiB)"

if [ "$CPU_MAX" = "100000 100000" ] && [ "$MEM_MAX" = "268435456" ]; then
  echo
  echo "RESULT: PASS - full-stack scaling request applied to the real cgroup"
  exit 0
else
  echo
  echo "RESULT: FAIL - cgroup values did not match the requested target"
  echo "----- NodeAgent log -----"; tail -20 "$NA_LOG" 2>/dev/null || true
  echo "----- ActionController log -----"; podman logs pullpiri-e2e-actioncontroller 2>&1 | tail -20 || true
  echo "----- ResourceManager log -----"; podman logs pullpiri-e2e-resourcemanager 2>&1 | tail -20 || true
  exit 1
fi
