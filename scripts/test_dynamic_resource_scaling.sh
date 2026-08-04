#!/usr/bin/env bash
#
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
#
# Dynamic Resource Scaling test helper (#514 / #526)
#
# Runs, in order:
#   1) Unit tests   - ResourceManager validation/reconcile + NodeAgent payload parsing
#   2) Live test    - Updates the CPU/Memory of a real running Podman container
#                     through the NodeAgent runtime path, then verifies with
#                     `podman inspect`.
#
# Requirements for the live test:
#   - podman available (Podman >= 4.3 for the container update endpoint).
#   - The script auto-detects the Podman API socket and, if it is not running,
#     starts a transient `podman system service` and stops it on exit.
#     Works for both root (rootful) and rootless Podman.
#     Override the socket explicitly with PODMAN_SOCKET=/path/to.sock if needed.
#
# Usage:
#   sudo scripts/test_dynamic_resource_scaling.sh       # root (rootful) podman
#   scripts/test_dynamic_resource_scaling.sh            # rootless podman
#   SKIP_LIVE=1 scripts/test_dynamic_resource_scaling.sh# unit tests only
#   IMAGE=docker.io/library/alpine scripts/test_dynamic_resource_scaling.sh

set -euo pipefail

# Make sure `cargo` is reachable, even under sudo where the caller's PATH and
# ~/.cargo/bin are not inherited. Look in the invoking user's home first.
if ! command -v cargo >/dev/null 2>&1; then
  for _c in \
    "${SUDO_USER:+/home/$SUDO_USER/.cargo/bin}" \
    "${HOME:+$HOME/.cargo/bin}" \
    "/root/.cargo/bin"; do
    if [ -n "$_c" ] && [ -x "$_c/cargo" ]; then
      export PATH="$_c:$PATH"
      # cargo is usually a rustup shim, so point rustup/cargo at the same
      # user's home to resolve the installed toolchain.
      _home="$(dirname "$(dirname "$_c")")"
      export RUSTUP_HOME="${RUSTUP_HOME:-$_home/.rustup}"
      export CARGO_HOME="${CARGO_HOME:-$_home/.cargo}"
      break
    fi
  done
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: 'cargo' not found. Install Rust or run with:" >&2
  echo "  sudo -E env \"PATH=\$PATH\" $0" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/src"
NODEAGENT="$SRC/agent/nodeagent"
CONTAINER="${CONTAINER:-pullpiri-scaling-test}"
IMAGE="${IMAGE:-docker.io/library/busybox}"

echo "======================================================================"
echo " Dynamic Resource Scaling - test suite (#514 / #526)"
echo "======================================================================"

echo
echo "== 1) Unit tests =========================================="
echo "-- ResourceManager (validation / desired-actual reconcile) --"
( cd "$SRC" && cargo test -p resourcemanager )
echo "-- NodeAgent (Podman update payload / error parsing) --"
( cd "$NODEAGENT" && cargo test runtime::podman::resource::tests )

if [ "${SKIP_LIVE:-0}" = "1" ]; then
  echo
  echo "SKIP_LIVE=1 set -> skipping live Podman integration test."
  exit 0
fi

echo
echo "== 2) Live Podman integration test ========================"
if ! command -v podman >/dev/null 2>&1 || ! podman info >/dev/null 2>&1; then
  echo "podman not available or not usable -> skipping live test."
  echo "(unit tests above already validate the logic.)"
  exit 0
fi

# --- Resolve the Podman API socket -----------------------------------------
# Priority: explicit PODMAN_SOCKET > path reported by `podman info` > default.
# If the socket is not present yet, start a transient API service and stop it
# on exit. Works for both root (rootful) and rootless Podman.
if [ -z "${PODMAN_SOCKET:-}" ]; then
  PODMAN_SOCKET="$(podman info --format '{{.Host.RemoteSocket.Path}}' 2>/dev/null || true)"
  [ -z "$PODMAN_SOCKET" ] && PODMAN_SOCKET="/run/podman/podman.sock"
fi
export PODMAN_SOCKET
echo "-- using Podman socket: $PODMAN_SOCKET --"

SVC_PID=""
if [ ! -S "$PODMAN_SOCKET" ]; then
  echo "-- socket not found, starting a transient Podman API service --"
  mkdir -p "$(dirname "$PODMAN_SOCKET")"
  podman system service --time=0 "unix://$PODMAN_SOCKET" \
    >/tmp/pullpiri-podman-svc.log 2>&1 &
  SVC_PID=$!
  for _ in $(seq 1 20); do
    [ -S "$PODMAN_SOCKET" ] && break
    sleep 0.5
  done
  if [ ! -S "$PODMAN_SOCKET" ]; then
    echo "failed to start Podman API service (see /tmp/pullpiri-podman-svc.log)"
    exit 1
  fi
fi

cleanup() {
  podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
  [ -n "$SVC_PID" ] && kill "$SVC_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "-- creating test container ($CONTAINER) with --cpus 2 --memory 512m --"
podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
podman run -d --name "$CONTAINER" --cpus 2 --memory 512m "$IMAGE" sleep 600 >/dev/null

echo "-- state BEFORE scaling --"
podman inspect "$CONTAINER" \
  --format 'NanoCpus={{.HostConfig.NanoCpus}} Memory={{.HostConfig.Memory}}'

echo "-- running NodeAgent update path (target: 1 core / 256 MiB) --"
SCALING_TEST_CONTAINER="$CONTAINER" PODMAN_SOCKET="$PODMAN_SOCKET" \
  bash -c "cd '$NODEAGENT' && cargo test runtime::podman::resource::tests::integration -- --ignored --nocapture"

echo "-- verifying the real container cgroup (authoritative) --"
CGP="$(podman inspect "$CONTAINER" --format '{{.State.CgroupPath}}')"
CPU_MAX="$(cat "/sys/fs/cgroup${CGP}/cpu.max" 2>/dev/null || echo 'n/a')"
MEM_MAX="$(cat "/sys/fs/cgroup${CGP}/memory.max" 2>/dev/null || echo 'n/a')"
echo "cpu.max    = ${CPU_MAX}   (expect: 100000 100000  -> 1 core)"
echo "memory.max = ${MEM_MAX}   (expect: 268435456      -> 256 MiB)"
if [ "${CPU_MAX}" = "100000 100000" ] && [ "${MEM_MAX}" = "268435456" ]; then
  echo "RESULT: PASS - runtime CPU/Memory applied to cgroup"
else
  echo "RESULT: FAIL - cgroup values do not match expected"
  exit 1
fi

echo
echo "All Dynamic Resource Scaling tests completed."
