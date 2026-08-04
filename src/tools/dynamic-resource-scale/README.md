<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# scaling_client

> 한국어 가이드: [README.ko.md](./README.ko.md)

`scaling_client` is a minimal command-line client for the Dynamic Resource
Scaling feature (#514 / #526). It sends a single `RequestResourceScaling` RPC
to a running ActionController, which then drives the ResourceManager and the
target NodeAgent to apply new CPU / memory limits to a running container at
runtime.

It is shipped as a Cargo *example* of the `actioncontroller` crate
(`src/player/actioncontroller/examples/scaling_client.rs`), so no separate
crate or manifest entry is required to build it.

## Building the binary

The client is built from the workspace under `src/`.

```bash
cd src

# Debug build (fast to compile):
cargo build -p actioncontroller --example scaling_client

# Release build (recommended for use):
cargo build -p actioncontroller --example scaling_client --release
```

The resulting binary is written to:

```
src/target/debug/examples/scaling_client      # debug build
src/target/release/examples/scaling_client     # release build
```

You may copy it to a location on your `PATH` for convenience, for example:

```bash
cp src/target/release/examples/scaling_client /usr/local/bin/scaling_client
```

> Note: the compiled binary is a build artifact and is not committed to the
> repository. Rebuild it with the commands above whenever the source changes.

### Running without building a standalone binary

For a one-off invocation you can let Cargo build and run it in one step:

```bash
cd src
cargo run -p actioncontroller --example scaling_client -- <args...>
```

## Usage

```
scaling_client <workload_id> <cpu> <mem_mib> [node] [endpoint]
```

| Argument      | Required | Description                                                         |
| ------------- | -------- | ------------------------------------------------------------------- |
| `workload_id` | yes      | Container name or id to resize (e.g. `helloworld_helloworld`).      |
| `cpu`         | yes      | Target CPU limit. See "CPU units" below.                            |
| `mem_mib`     | yes      | Target memory limit in MiB (integer).                               |
| `node`        | no       | Target node as a hostname (e.g. `HPC`) or an IP address. See "Targeting a node" below. Defaults to the local node. |
| `endpoint`    | no       | ActionController gRPC endpoint. Defaults to `http://<host.ip>:47001`, where `host.ip` is read from the local `/etc/pullpiri/settings.yaml`. |

### CPU units

CPU is expressed in millicores, where `1000m == 1 core`. The `cpu` argument
accepts either whole/fractional cores or a Kubernetes-style millicore suffix:

| Input   | Meaning        |
| ------- | -------------- |
| `1`     | 1 core (1000m) |
| `0.5`   | 500m           |
| `500m`  | 500m           |
| `2000m` | 2 cores        |
| `2`     | 2 cores        |

### Targeting a node

The `node` argument may be either an IP address or a node hostname. Resolution
is performed server-side by the ActionController:

- Empty -> the local node (`127.0.0.1`).
- An IP literal (e.g. `10.231.176.123`) -> used as-is.
- A hostname (e.g. `HPC`) -> resolved to an IP by looking it up in the cluster
  node registry (`cluster/nodes/` in the key-value store). This lets you resize
  a workload running on a remote node by its node name.

If a hostname cannot be resolved (registry unreachable or no matching node),
the value is passed through unchanged so the resulting connection error is
still meaningful.

### Selecting the ActionController endpoint

The client always runs on the local (master) node, so the ActionController
endpoint does not normally need to be supplied. When `endpoint` is omitted it is
built as `http://<host.ip>:47001`, reading `host.ip` from the local
`/etc/pullpiri/settings.yaml` (falling back to `127.0.0.1` when unset or
`0.0.0.0`). Pass an explicit `endpoint` only to target a non-local
ActionController.

Note that `endpoint` (the ActionController the client talks to) and `node` (the
node whose workload is resized) are distinct: the client always connects to a
single ActionController, which then routes the runtime update to the target
node's NodeAgent.

### Scale direction

There is no `up` / `down` argument. The scale direction is decided entirely on
the server side: the ResourceManager compares the request against the current
desired state per resource and only capacity-checks a resource that increases.
A resource that stays the same or shrinks is never rejected, and a caller
cannot bypass the capacity check by mislabelling the request.

## Examples

```bash
# Resize container "helloworld_helloworld" to 0.5 core / 64 MiB on node "HPC".
# The endpoint is taken from the local settings file.
scaling_client helloworld_helloworld 0.5 64 HPC

# Same request using the millicore suffix.
scaling_client helloworld_helloworld 500m 64 HPC

# Target a node by IP instead of hostname.
scaling_client my-workload 2 512 192.168.0.10

# Override the ActionController endpoint explicitly (non-local master).
scaling_client my-workload 2 512 192.168.0.10 http://192.168.0.20:47001
```

Successful output looks like:

```
-> RequestResourceScaling endpoint=http://10.231.176.123:47001 node='HPC' workload='helloworld_helloworld' cpu=500m mem=64MiB
<- success        : true
<- message        : scaling applied and synchronized
<- sync_state     : 3
<- actual_cpu     : 500m
<- actual_memory  : 64 MiB
```

## Prerequisites

The following services must be running and reachable before invoking the client:

- ActionController (default gRPC port `47001`) - the endpoint this client connects to.
- ResourceManager (default gRPC port `47008`) - performs availability validation and owns the desired state.
- NodeAgent (default gRPC port `47004`) - applies the runtime update through the Podman REST API.

The NodeAgent must be able to reach the Podman socket of the runtime that owns
the target container. For a container running under the root Podman store, run
the NodeAgent with access to `/run/podman/podman.sock` (for example, as root).

## Verifying the applied limits

Podman does not reflect a runtime `update` in `inspect`; the authoritative
values are in the container cgroup. For a container running under the root
Podman store on cgroup v2:

```bash
# Resolve the cgroup scope from a process running inside the container, then
# read the limits (1 core == "100000 100000", 64 MiB == 67108864 bytes).
scope=/sys/fs/cgroup/machine.slice/libpod-<CONTAINER_ID>.scope
cat "$scope/cpu.max"      # e.g. "50000 100000"  -> 0.5 core
cat "$scope/memory.max"   # e.g. "67108864"      -> 64 MiB
```

## Setting initial limits in the workload manifest

`scaling_client` changes the limits of an *already running* container at
runtime. To give a workload a resource limit from the moment it is created,
declare it in the workload manifest instead. The NodeAgent reads
`spec.containers[].resources.limits` when it creates the container and applies
the CPU / memory limits through the Podman API.

Add a `resources.limits` block to the container entry in the `Model` document.
Using `examples/resources/helloworld_no_condition.yaml` as an example, the
`Model` section looks like this:

```yaml
apiVersion: v1
kind: Model
metadata:
  name: helloworld
  annotations:
    io.pullpiri.annotations.package-type: helloworld
    io.pullpiri.annotations.package-name: helloworld
    io.pullpiri.annotations.package-network: default
  labels:
    app: helloworld
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: quay.io/podman/hello:latest
      resources:              # initial limits applied at container creation
        limits:
          cpu: "0.2"          # cores, as a string: "0.2" -> 0.2 core
          memory: "4Mi"       # Ki / Mi / Gi (x1024) or K / M / G (x1000)
  terminationGracePeriodSeconds: 0
```

Field reference for `resources.limits`:

| Field    | Format                                                                  | Example  |
| -------- | ----------------------------------------------------------------------- | -------- |
| `cpu`    | String, whole or fractional cores. Parsed as `f64` into `NanoCpus`.     | `"0.2"`, `"1"`, `"2"` |
| `memory` | String, plain bytes or a `Ki`/`Mi`/`Gi` (x1024) or `K`/`M`/`G` (x1000) suffix. | `"4Mi"`, `"512Mi"`, `"1Gi"` |

Notes:

- Both values are strings (quote them).
- `cpu` here uses cores (not the millicore form used by `scaling_client`);
  `"0.5"` means half a core.
- These limits apply only at container creation. To change them on a running
  container, either recreate the workload or use `scaling_client`.

