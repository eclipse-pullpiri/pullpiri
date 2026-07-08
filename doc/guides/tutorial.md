<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Tutorial: Run Your First Scenario

This tutorial walks you through deploying and running the built-in `helloworld` example scenario that is included in the `examples/` directory. By the end, you will understand Pullpiri's core resource model and how to submit and monitor a workload.

> **Back to:** [Getting Started](./getting-started.md)  
> **Prerequisite:** Pullpiri must be installed and all containers must be in `Running` state. Follow the [Quick Start](./getting-started.md#quick-start) or [Build from Source](./build.md) guide first. Ensure the required ports (8080, 47001–47007, 47098–47099) are open in your firewall — see [Open Required Ports](./getting-started.md#open-required-ports).

---

## Background: The Resource Model

Every workload in Pullpiri is described by a stack of three Kubernetes-style resources defined in a single YAML file:

```
Scenario  ──▶  defines trigger condition + action
   └── Package  ──▶  groups Models and defines deployment pattern
         └── Model  ──▶  defines the actual container workload (pod spec)
```

| Resource | Role |
|----------|------|
| `Scenario` | Declares **when** to act (`condition`) and **what** to do (`action: launch / update / delete`) |
| `Package` | Groups one or more `Model` resources and specifies the node(s) to deploy to |
| `Model` | Kubernetes Pod-like spec describing the container image, network, restart policy, etc. |

---

## Step 1: Verify Pullpiri is Running

Before running a scenario, confirm that all Pullpiri services are healthy:

```bash
podman pod ps
# NAME              STATUS
# pullpiri-server   Running
# pullpiri-player   Running
```

Verify the nodeagent systemd service:

```bash
systemctl status nodeagent.service
# ● nodeagent.service - Pullpiri NodeAgent Service
#    Active: active (running) ...
```

Check the API server log:

```bash
podman logs pullpiri-apiserver
# http api listening on 0.0.0.0:47099
```

Check the player gateway log:

```bash
podman logs pullpiri-filtergateway
# FilterGatewayManager init
# Pullpirid gateway listening on 0.0.0.0:47002
```

---

## Step 2: Examine the Example Scenario

Navigate to the `examples/resources/` directory:

```bash
ls examples/resources/
# helloworld.yaml               (with DDS condition)
# helloworld_no_condition.yaml  (no condition, launches immediately)
# helloworld_policy.yaml        (with policy)
# parameter-test.yaml
# schedule-test.yaml
```

For this tutorial we use `helloworld_no_condition.yaml`, which launches the workload immediately without requiring a DDS signal condition.

View the file:

```bash
cat examples/resources/helloworld_no_condition.yaml
```

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: null          # No trigger condition → deploy immediately
  action: launch           # Action: launch the Package
  target: helloworld       # Name of the Package to launch
---
apiVersion: v1
kind: Package
metadata:
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld
      node: HPC            # ← Replace with your node hostname
      resources:
        volume:
        network:
---
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
      image: quay.io/podman/hello:latest   # Simple hello-world container
  terminationGracePeriodSeconds: 0
  restartPolicy: Always
```

> **Note:** The `node` field in `Package.spec.models[].node` must match your host's hostname (or the name in `/etc/pullpiri/settings.yaml`).

---

## Step 3: Set Your Node Name

Check your current hostname:

```bash
hostname
# HPC   (example)
```

If your hostname differs from `HPC`, update the `node` field in the YAML file:

```bash
# Replace HPC with your actual hostname
sed -i "s/node: HPC/node: $(hostname)/" examples/resources/helloworld_no_condition.yaml
```

Verify the change:

```bash
grep "node:" examples/resources/helloworld_no_condition.yaml
# node: <your-hostname>
```

---

## Step 4: Submit the Scenario

### Option A – Use the Provided Shell Script

The `examples/` directory includes a ready-to-use script:

```bash
cd examples/
bash helloworld.sh
```

This script automatically detects the host IP and submits the scenario via the REST API:

```bash
# helloworld.sh contents:
# BODY=$(< ./resources/helloworld_no_condition.yaml)
# HOST_IP=$(hostname -I | awk '{print $1}')
# curl -X POST "http://${HOST_IP}:47099/api/artifact" \
#   --header 'Content-Type: text/plain' \
#   --data "${BODY}"
```

### Option B – Submit Manually with curl

```bash
HOST_IP=$(hostname -I | awk '{print $1}')

curl -X POST "http://${HOST_IP}:47099/api/artifact" \
  --header 'Content-Type: text/plain' \
  --data-binary @examples/resources/helloworld_no_condition.yaml
```

A successful submission returns `200 OK`.

---

## Step 5: Verify the Workload is Running

After a few seconds, check that the container has been started by `nodeagent`:

```bash
podman ps
# CONTAINER ID  IMAGE                          COMMAND  ...  NAMES
# 39ab0e9e945f  quay.io/podman/hello:latest    ...       ...  helloworld-helloworld
```

Check the container logs:

```bash
podman logs helloworld-helloworld
```

Expected output:

```
!... Hello Podman World ...!

         .--"--.
       / -     - \
      / (O)   (O) \
   ~~~| -=(,Y,)=- |
    .---. /`  \   |~~
 ~/  o  o \~~~~.----. ~~
  | =(X)= |~  / (O (O) \
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
```

> **Note:** Since `restartPolicy: Always` is set, this container restarts continuously. This is expected behavior for the helloworld example.

---

## Step 6: Monitor via API Server

You can also query the current state of all submitted artifacts through the API server.

> **Note:** The API server does not expose individual `/api/scenario` or `/api/package` GET endpoints. Use the `GET /api/notify` endpoint to check connectivity, and query stored data via the RocksDB inspector tool (`src/tools/rocksdb-inspector`) or by checking service logs.

```bash
HOST_IP=$(hostname -I | awk '{print $1}')

# Verify API server is reachable
curl -X GET "http://${HOST_IP}:47099/api/notify"

# Submit a new artifact
curl -X POST "http://${HOST_IP}:47099/api/artifact" \
  --header 'Content-Type: text/plain' \
  --data-binary @examples/resources/helloworld_no_condition.yaml

# Check apiserver logs for artifact processing
podman logs pullpiri-apiserver

# Check policymanager logs
podman logs pullpiri-policymanager
```

---

## Step 7: Try a Scenario with a Condition

The `helloworld.yaml` file demonstrates a **conditional** scenario: the workload is launched only when a DDS signal matches the specified condition.

```bash
cat examples/resources/helloworld.yaml
```

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: value
      value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld
```

This scenario:
- Listens for the DDS topic `ADASObstacleDetectionIsWarning`
- Triggers the `update` action on the `helloworld` Package when the signal value equals `"true"`

To submit this scenario (requires a running DDS environment):

```bash
HOST_IP=$(hostname -I | awk '{print $1}')

curl -X POST "http://${HOST_IP}:47099/api/artifact" \
  --header 'Content-Type: text/plain' \
  --data-binary @examples/resources/helloworld.yaml
```

---

## Step 8: Remove the Scenario

To stop and remove the workload, send the **same YAML** via `DELETE /api/artifact`.
The API server searches for a `Scenario` kind in the body and removes it:

```bash
HOST_IP=$(hostname -I | awk '{print $1}')

curl -X DELETE "http://${HOST_IP}:47099/api/artifact" \
  --header 'Content-Type: text/plain' \
  --data-binary @examples/resources/helloworld_no_condition.yaml
```

> **Note:** The DELETE endpoint accepts the whole YAML body (same format as POST).
> It extracts the `Scenario` resource from it and removes it from storage.
> There is no URL-path-based `DELETE /api/scenario/{name}` endpoint.

Verify the container has been removed:

```bash
podman ps
# (helloworld-helloworld should no longer appear)
```

---

## Summary

In this tutorial you:

1. Verified that Pullpiri services are running.
2. Examined the three-tier resource model: Scenario → Package → Model.
3. Submitted the `helloworld` scenario using the REST API.
4. Verified the container workload was started by the nodeagent.
5. Learned about conditional scenarios using DDS signal triggers.
6. Removed the scenario and confirmed the workload was stopped.

---

## Next Steps

- Explore more example scenarios in `examples/resources/`
- Read the [API Reference](./pullpiri-apis.md) to learn all available REST endpoints
- Read the [Project Structure](./structure.md) to understand how each component works
- Check [Development Guide](./developments.md) to start contributing

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `curl` returns connection refused | API server not running | Check `podman logs pullpiri-apiserver` |
| Container not starting after scenario submit | Node name mismatch | Ensure `node` in YAML matches `hostname` output |
| Container keeps restarting | `restartPolicy: Always` is set | Expected for helloworld demo; change to `Never` if needed |
| `podman ps` shows no new container | nodeagent not running | Check `systemctl status nodeagent.service` or `journalctl -u nodeagent.service` |
