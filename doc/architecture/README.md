<!--
SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri Architecture

This directory contains architecture documentation for the Pullpiri vehicle service orchestrator. It covers both high-level design (HLD) and low-level design (LLD) documents, as well as descriptions of the core resource types used by Pullpiri.

## Directory Structure

```
architecture/
├── README.md           # This file
├── package.md          # Package resource specification
├── scenario.md         # Scenario resource specification
├── EN/                 # English architecture documents
│   ├── HLD/            # High-Level Design
│   │   ├── pullpiri_clustering.md
│   │   └── settingsservice.md
│   └── LLD/            # Low-Level Design
│       ├── clustering.md
│       └── settingsservice.md
└── KR/                 # Korean architecture documents
    ├── HLD/            # High-Level Design (한국어)
    │   ├── pullpiri_clustering.md
    │   └── settingsservice.md
    └── LLD/            # Low-Level Design (한국어)
        ├── clustering.md
        ├── settingsservice.md
        ├── statemachine.md
        ├── StateManager_Model.md
        ├── StateManager_Package.md
        ├── StateManager_Scenario.md
        ├── StateManager_review.md
        ├── 1.NodeAgent_DesiredState_Storage.md
        ├── 2.NodeAgent_Reconciliation_Loop.md
        ├── 3.NodeAgent_Liveness_Probe.md
        └── 4.NodeAgent_Restart_Policy.md
```

## Core Resources

Pullpiri manages vehicle services through two primary resource types: **Package** and **Scenario**.

### Package

A package represents a vehicle service, bundling container image information, network settings, and volume requirements in tar format. Separating these resources allows users to compose services flexibly.

See [package.md](./package.md) for the full specification.

### Scenario

A scenario defines when and how a package is executed. It consists of three components:

- **Condition** — the trigger or state that must be met
- **Action** — the operation to perform (e.g., create, update, delete)
- **Target** — the package to act upon

See [scenario.md](./scenario.md) for the full specification.

## Design Documents

### High-Level Design (HLD)

Describes overall system architecture and component interactions.

| Document | EN | KR |
|---|---|---|
| Pullpiri Clustering | [EN](./EN/HLD/pullpiri_clustering.md) | [KR](./KR/HLD/pullpiri_clustering.md) |
| Settings Service | [EN](./EN/HLD/settingsservice.md) | [KR](./KR/HLD/settingsservice.md) |

### Low-Level Design (LLD)

Describes internal implementation details of individual components.

| Document | EN | KR |
|---|---|---|
| Clustering | [EN](./EN/LLD/clustering.md) | [KR](./KR/LLD/clustering.md) |
| Settings Service | [EN](./EN/LLD/settingsservice.md) | [KR](./KR/LLD/settingsservice.md) |
| State Machine | — | [KR](./KR/LLD/statemachine.md) |
| StateManager (Model/Package/Scenario) | — | [KR](./KR/LLD/StateManager_Model.md) |
| NodeAgent (DesiredState / Reconciliation / Liveness / Restart) | — | [KR](./KR/LLD/1.NodeAgent_DesiredState_Storage.md) |
