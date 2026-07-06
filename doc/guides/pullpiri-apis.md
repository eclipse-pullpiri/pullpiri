<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri REST API

Pullpiri REST API provides an HTTP-based interface for deploying and managing artifacts from the cloud or other systems to vehicle nodes.

**API Server Address**: `http://<host>:47099`

## Overview

Pullpiri API provides the following main features:

- **Deploy Artifacts** (POST /api/artifact): Deploy artifacts such as Scenario, Package, Model, etc.
- **Withdraw Artifacts** (DELETE /api/artifact): Remove deployed artifacts
- **Deployment Notification** (GET /api/notify): Receive notifications of new artifact releases from the cloud

## Endpoints

### 1. Deploy Artifacts

Deploy new artifacts (Scenario, Package, Model, Volume, Network, Node, Schedule, Policy).

```
POST /api/artifact
```

#### Request Headers

| Header | Value |
|--------|-------|
| Content-Type | text/plain (or application/x-yaml) |

#### Request Body

Artifact definitions in YAML format. Multiple artifacts are separated by `---` delimiter.

##### Example: Deploy a Scenario

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: ""
  action: update
  target: helloworld
```

##### Example: Deploy Package and Model together

```yaml
apiVersion: v1
kind: Package
metadata:
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources: {}
      volume: {}
      network: {}
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.pullpiri.annotations.package-type: helloworld-core
    io.pullpiri.annotations.package-name: helloworld
    io.pullpiri.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld:latest
  terminationGracePeriodSeconds: 0

##### Success (200 OK)

```json
"Ok"
```

##### Failure (405 or other error)

```json
"Error message describing the failure"
```

#### Response Status Codes

| Code | Description |
|------|-------------|
| 200 | Artifact deployment successful |
| 405 | Invalid request or processing error |

---

### 2. Withdraw Artifacts

Remove deployed artifacts.

```
DELETE /api/artifact
```

#### Request Headers

| Header | Value |
|--------|-------|
| Content-Type | text/plain |

#### Request Body

Name of the artifact to be removed (string)

##### Example

```
helloworld
```

#### Response

##### Success (200 OK)

```json
"Ok"
```

##### Failure (405 or other error)

```json
"Error message describing the failure"
```

#### Response Status Codes

| Code | Description |
|------|-------------|
| 200 | Artifact withdrawal successful |
| 405 | Invalid request or processing error |

---

### 3. Deployment Notification

Notify the API server that a new artifact has been released from the cloud.

```
GET /api/notify
```

#### Request Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| artifact_name | String | Yes | Name of the newly released artifact |

#### Example

```
GET /api/notify?artifact_name=helloworld
```

#### Response

##### Success (200 OK)

```json
"Ok"
```

##### Failure (405 or other error)

```json
"Error message describing the failure"
```

#### Response Status Codes

| Code | Description |
|------|-------------|
| 200 | Notification received successfully |
| 405 | Invalid HTTP method |

---

## Artifact Types

The following artifact types are supported by the Pullpiri API:

| Kind | Description |
|------|-------------|
| Scenario | Action definition to be executed under specific conditions |
| Package | Definition of an application package to be deployed |
| Model | Container model defined with Kubernetes Pod specification |
| Volume | Storage volume definition |
| Network | Network configuration definition |
| Node | Node information definition |
| Schedule | Scheduled task definition |
| Policy | Policy rule definition |

---

## Usage Examples

### Deploy a Scenario using cURL

```bash
curl -X POST http://localhost:47099/api/artifact \
  -H "Content-Type: text/plain" \
  -d @helloworld_scenario.yaml
```

### Withdraw an Artifact using cURL

```bash
curl -X DELETE http://localhost:47099/api/artifact \
  -H "Content-Type: text/plain" \
  -d "helloworld"
```

### Send a Deployment Notification using cURL

```bash
curl -X GET "http://localhost:47099/api/notify?artifact_name=helloworld"
```

---

## Error Handling

All API responses indicate success or failure using HTTP status codes:

- **200 OK**: Request processed successfully
- **405 Method Not Allowed**: The HTTP method is not allowed or an error occurred during processing

The error response body contains a detailed error message.

---

## Notes

- All requests must be in valid YAML or string format.
- When deploying multiple artifacts, separate each artifact with the YAML delimiter (`---`).
- The API server stores received artifacts in RocksDB and forwards them to other components such as the filter gateway.