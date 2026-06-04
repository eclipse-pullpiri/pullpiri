<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->

# Baseline Management Rules

**Last Updated**: 2026-05-22

**Created On**: 2026-04-30  
**Author**: Pullpiri CM Team

## Table of Contents

1. [Overview](#1-overview)
2. [Baseline Tag Naming Convention](#2-baseline-tag-naming-convention)
3. [Baseline Types](#3-baseline-types)
4. [Baseline Criteria](#4-baseline-criteria)
5. [Baseline Procedure](#5-baseline-procedure)
6. [Change Management After Baseline](#6-change-management-after-baseline)
7. [Roles and Stakeholders](#7-roles-and-stakeholders)
8. [Baseline Management Diagrams](#8-baseline-management-diagrams)

---

## 1. Overview

A baseline is a reference point that represents a specific point in the project development lifecycle, and it is established by the CM (Configuration Management) manager.

If any code changes occur after a baseline is established, those changes must go through the stakeholder **approval and notification** process.

## 2. Baseline Tag Naming Convention

Pullpiri follows its own convention based on [Semantic Versioning 2.0.0](https://semver.org/).

```text
v<MAJOR>.<MINOR>.<PATCH>[-<identifier>]
```

| Element      | Description                                                                 |
| ------------ | --------------------------------------------------------------------------- |
| `MAJOR`      | Incremented for incompatible changes intended for external organization delivery |
| `MINOR`      | Incremented when backward-compatible features are added                     |
| `PATCH`      | Incremented for backward-compatible bug fixes                               |
| `identifier` | Additional unofficial baseline qualifier (alpha, beta, rc1, milestone1, etc.) |

### Examples

| Tag                 | Meaning                                              |
| ------------------- | ---------------------------------------------------- |
| `v1.0.0`            | First official release                               |
| `v1.1.0`            | Official release with backward-compatible new features |
| `v1.1.1`            | Official patch release for bug fixes                 |
| `v2.0.0`            | Official major release with incompatible changes     |
| `v1.2.0-alpha`      | Unofficial baseline for internal alpha testing       |
| `v1.2.0-rc1`        | Unofficial baseline for release candidate            |
| `v1.2.0-milestone1` | Unofficial baseline for internal milestone           |

## 3. Baseline Types

### 3.1 Major Baseline

- **Definition**: A baseline for achieving internal LGE goals or marking development milestones
- **Target Branch**: `main`
- **Created By**: CM manager
- **Approval Requirement**: Approval from all stakeholders is required
- **GitHub Tag Format**: `v<MAJOR>.<MINOR>.<PATCH>` (e.g., `v1.0.0`)
- **GitHub Release**: When establishing an official baseline, release notes must be created on the GitHub Release page

### 3.2 Minor Baseline

- **Definition**: A baseline for internal review, pre-release validation, or incorporating minor, non-functional code updates
- **Target Branch**: `main` or a specific feature branch
- **Created By**: CM manager or development lead
- **Approval Requirement**: Approval from relevant team members
- **GitHub Tag Format**: `v<MAJOR>.<MINOR>.<PATCH>` (e.g., `v1.1.0`, `v1.2.1`)

### 3.3 Unofficial Baseline

- **Definition**: A baseline to urgently reflect fixes for purposes unrelated to functional safety, within a scope that handles non-safety domains. As a principle, it should be discarded after the objective is achieved.
- **Target Branch**: A specific feature branch
- **Created By**: CM manager
- **Approval Requirement**: CM manager approval
- **GitHub Tag Format**: `v<MAJOR>.<MINOR>.<PATCH>-<identifier>` (e.g., `v1.0.0-alpha`, `v1.1.0-milestone1`)

## 4. Baseline Criteria

### 4.1 Criteria for Major Baseline

An official baseline is established only when all of the following conditions are met.

- Internal milestone reached (end of sprint, transition of development phase, etc.)
- Development and integration of a major feature branch completed

### 4.2 Criteria for Minor Baseline

An unofficial baseline is established when one or more of the following conditions are met.

- Non-functional code changes and minor routine updates
- Snapshot needed for internal review or demo
- Pre-validation (RC, alpha, beta) required before external release

## 5. Baseline Procedure

<img width="1146" height="641" alt="image" src="https://github.com/user-attachments/assets/5e4e3bf9-98d2-4aa4-9054-1ebc053f70ea" />

## 6. Change Management After Baseline

<img width="1534" height="920" alt="image" src="https://github.com/user-attachments/assets/b5585fc9-6235-4c99-9688-91c0d1a8cac2" />

## 7. Roles and Stakeholders

| Role             | Responsibility                                                       |
| ---------------- | -------------------------------------------------------------------- |
| **CM Manager**   | Establish baseline, create tags, send notifications, lead intake and impact analysis of change requests |
| **Dev Lead**     | Verify feature completion and quality criteria, approve unofficial baselines |
| **Stakeholders** | Approve official baseline setup and changes                          |
| **Developers**   | Register change issues, implement changes, and create PRs            |

## 8. Baseline Management Diagrams

### Baseline Setup Flow

```text
Development complete and tests passed
        ↓
CM pre-check (verify criteria items)
        ↓
Open baseline setup issue
        ↓
Obtain stakeholder approval
        ↓
Create and push Git tag
   - Official: v<MAJOR>.<MINOR>.<PATCH>
   - Unofficial: v<MAJOR>.<MINOR>.<PATCH>-<identifier>
        ↓
Write GitHub Release (official baseline only)
        ↓
Notify stakeholders
        ↓
Close baseline setup issue
```

### Post-baseline Change Flow

```text
Change requirement identified
        ↓
Create change issue (BUG/FEATURE)
        ↓
Impact analysis (CM manager + developers)
        ↓
Stakeholder approval
        ↓
Implement change in feature branch
        ↓
Create PR → Review → CI pass
        ↓
Merge into main branch
        ↓
Set new baseline (if needed)
        ↓
Notify stakeholders
```

### Version History Example

```text
main ──●──────●────────────────────────────────────►
       │      │
     v1.0.0  v1.1.0
  (Official BL) (Official BL)
       │
       └──► model/vehicle-x1 (model branch)
```
