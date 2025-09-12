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

### Create Node

- **Endpoint:** `POST /nodes`
- **Description:** Create a new node.
- **Request Body:** `CreateNodeRequest`
- **Response:** `NodeInfo`
- **Status Codes:** 200, 400, 500

### Get Node Details

- **Endpoint:** `GET /nodes/{name}`
- **Description:** Get specific node details including logs.
- **Response:** `NodeInfo` (with logs)
- **Status Codes:** 200, 404, 500

### Delete Node

- **Endpoint:** `DELETE /nodes/{name}`
- **Description:** Delete a specific node.
- **Response:** None
- **Status Codes:** 204, 500

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

### Create SoC

- **Endpoint:** `POST /socs`
- **Description:** Create a new SoC.
- **Request Body:** `CreateSocRequest`
- **Response:** `SocInfo`
- **Status Codes:** 200, 400, 500

### Get SoC Details

- **Endpoint:** `GET /socs/{name}`
- **Description:** Get specific SoC details including logs.
- **Response:** `SocInfo` (with logs)
- **Status Codes:** 200, 404, 500

### Delete SoC

- **Endpoint:** `DELETE /socs/{name}`
- **Description:** Delete a specific SoC.
- **Response:** None
- **Status Codes:** 204, 500

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

### Create Board

- **Endpoint:** `POST /boards`
- **Description:** Create a new board.
- **Request Body:** `CreateBoardRequest`
- **Response:** `BoardInfo`
- **Status Codes:** 200, 400, 500

### Get Board Details

- **Endpoint:** `GET /boards/{name}`
- **Description:** Get specific board details including logs.
- **Response:** `BoardInfo` (with logs)
- **Status Codes:** 200, 404, 500

### Delete Board

- **Endpoint:** `DELETE /boards/{name}`
- **Description:** Delete a specific board.
- **Response:** None
- **Status Codes:** 204, 500

---

## Monitoring Integration

### Synchronize with Monitoring Server

- **Endpoint:** `POST /monitoring/sync`
- **Description:** Synchronize with monitoring server.
- **Response:** `SuccessResponse`
- **Status Codes:** 200, 500

---

## Request Body Schemas

### CreateNodeRequest

```json
{
  "name": "string",
  "ip": "string",
  "image": "string",
  "labels": {
    "environment": "string",
    "region": "string",
    "type": "string"
  }
}
```

### CreateSocRequest

```json
{
  "name": "string",
  "description": "string",
  "labels": {
    "architecture": "string",
    "vendor": "string",
    "generation": "string"
  }
}
```

### CreateBoardRequest

```json
{
  "name": "string",
  "description": "string",
  "labels": {
    "function": "string",
    "criticality": "string",
    "location": "string"
  }
}
```

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

## Example Usage

See the "Example Usage" section in the original documentation for sample `curl` commands.

---

## Data Storage

- **etcd** at `localhost:2379`
- **Data Paths:**
  - Nodes: `/piccolo/metrics/nodes/{node_name}`
  - SoCs: `/piccolo/metrics/socs/{soc_id}`
  - Boards: `/piccolo/metrics/boards/{board_id}`
  - Logs: `/piccolo/logs/{type}/{id}`
  - Metadata: `/piccolo/metadata/{type}/{id}`

---

## Error Handling

- Input validation with descriptive error messages
- etcd connection error handling
- Resource not found scenarios
- Serialization/deserialization error handling

---

## Development and Testing

Follow Pullpiri coding guidelines and validation