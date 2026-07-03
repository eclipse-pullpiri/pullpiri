<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Board/SoC/Node Management REST API Documentation

## Overview

The Pullpiri Settings Service provides REST APIs for managing Board, SoC (System on Chip), and Node resources. These APIs integrate with the monitoring server's etcd storage to provide real-time data and management capabilities.

---

## Base URL

```
http://localhost:8080/api/v1
```

---

## Node Management

### List Nodes

- **Endpoint:** `GET /nodes`
- **Description:** List all nodes with optional filtering and pagination.
- **Query Parameters:**
  - `page` (integer, default: 1)
  - `page_size` (integer, default: 50)
  - `filter` (string, optional)
- **Response:** `NodeListResponse`
- **Status Codes:** 200, 500

### Get Node Details

- **Endpoint:** `GET /nodes/{name}`
- **Description:** Get specific node details including logs.
- **Response:** `NodeInfo` (with logs)
- **Status Codes:** 200, 404, 500

### Get Pod Metrics for Node

- **Endpoint:** `GET /nodes/{name}/pods/metrics`
- **Description:** Get pod metrics for a specific node.
- **Query Parameters:**
  - `page_size` (integer, default: 100)
- **Response:** Array of `Metric`
- **Status Codes:** 200, 404, 500

---

## SoC Management

### List SoCs

- **Endpoint:** `GET /socs`
- **Description:** List all SoCs with optional filtering and pagination.
- **Query Parameters:**
  - `page` (integer, default: 1)
  - `page_size` (integer, default: 50)
  - `filter` (string, optional)
- **Response:** `SocListResponse`
- **Status Codes:** 200, 500

### Get SoC Details

- **Endpoint:** `GET /socs/{name}`
- **Description:** Get specific SoC details including logs.
- **Response:** `SocInfo` (with logs)
- **Status Codes:** 200, 404, 500

---

## Board Management

### List Boards

- **Endpoint:** `GET /boards`
- **Description:** List all boards with optional filtering and pagination.
- **Query Parameters:**
  - `page` (integer, default: 1)
  - `page_size` (integer, default: 50)
  - `filter` (string, optional)
- **Response:** `BoardListResponse`
- **Status Codes:** 200, 500

### Get Board Details

- **Endpoint:** `GET /boards/{name}`
- **Description:** Get specific board details including logs.
- **Response:** `BoardInfo` (with logs)
- **Status Codes:** 200, 404, 500

---

## Monitoring Integration

### Synchronize with Monitoring Server

- **Endpoint:** `POST /monitoring/sync`
- **Description:** Synchronize with monitoring server.
- **Response:** `SuccessResponse`
- **Status Codes:** 200, 500

---

## Response Schemas

- See the original documentation for `NodeInfo`, `SocInfo`, `BoardInfo`, `NodeListResponse`, `SocListResponse`, `BoardListResponse`, `SuccessResponse`, and `ErrorResponse` JSON structures.

---

## Status Codes

- **200**: OK (successful GET, POST)
- **204**: No Content (successful DELETE)
- **400**: Bad Request (invalid request body or missing fields)
- **404**: Not Found (resource not found)
- **500**: Internal Server Error (server-side errors, etcd issues)

---

## Data Storage

- **etcd** at `localhost:2379`
- **Data Paths:**
  - Nodes: `/pullpiri/metrics/nodes/{node_name}`
  - SoCs: `/pullpiri/metrics/socs/{soc_id}`
  - Boards: `/pullpiri/metrics/boards/{board_id}`
  - Logs: `/pullpiri/logs/{type}/{id}`
  - Metadata: `/pullpiri/metadata/{type}/{id}`

---

## Error Handling

- Input validation with descriptive error messages
- etcd connection error handling
- Resource not found scenarios
- Serialization/deserialization error handling

---

## Development and Testing

Follow Pullpiri coding guidelines and validation