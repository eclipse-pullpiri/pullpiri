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

- **Definition**: A baseline for release to external organizations (customers, partners, etc.)
- **Target Branch**: `main`
- **Created By**: CM manager
- **Approval Requirement**: Approval from all stakeholders is required
- **GitHub Tag Format**: `v<MAJOR>.<MINOR>.<PATCH>` (e.g., `v1.0.0`)
- **GitHub Release**: When establishing an official baseline, release notes must be created on the GitHub Release page

### 3.2 Minor Baseline

- **Definition**: A baseline for achieving internal LGE goals or marking development milestones
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

| Condition              | Details                                                                 |
| ---------------------- | ----------------------------------------------------------------------- |
| Feature implementation completed | All required features (FEATURE issues) for the release are implemented and merged via PR |
| Tests passed           | Both unit and integration tests are passed (`test:passed` label verified) |
| Code review completed  | At least one reviewer approval completed for all changes                |
| Build successful       | CI pipeline build success verified                                      |
| Documentation updated  | Related documents updated (`CHANGELOG`, `README`, API docs, etc.)      |
| Stakeholder approval   | Written (or issue-based) approval from all stakeholders before external release |

### 4.2 Criteria for Minor Baseline

An unofficial baseline is established when one or more of the following conditions are met.

- Internal milestone reached (end of sprint, transition of development phase, etc.)
- Development and integration of a major feature branch completed
- Snapshot needed for internal review or demo
- Pre-validation (RC, alpha, beta) required before external release

## 5. Baseline Procedure

### 5.1 Major Baseline Procedure

1. **Pre-check**: CM manager verifies all items in [4.1 Criteria for Major Baseline](#41-criteria-for-major-baseline)

2. **Issue creation**: Create a baseline issue on GitHub

   - Title: `[TASK] Set official baseline for v<version>`
   - Labels: `type:task`, `priority:critical`

3. **Stakeholder approval**: Obtain stakeholder approval through the issue or a separate channel

4. **Create and push tag**:

   ```bash
   git tag v<MAJOR>.<MINOR>.<PATCH>
   git push origin v<MAJOR>.<MINOR>.<PATCH>
   ```

5. **Write GitHub Release**: Create release notes for the tag on the GitHub Release page

6. **Notification**: Notify all stakeholders and the development team that baseline setup is complete

7. **Issue close**: Close the baseline setup issue

### 5.2 Unofficial Baseline Procedure

1. **Pre-check**: Confirm approval from relevant team members

2. **Create and push tag**:

   ```bash
   git tag v<MAJOR>.<MINOR>.<PATCH>-<identifier>
   git push origin v<MAJOR>.<MINOR>.<PATCH>-<identifier>
   ```

3. **Notification**: Notify relevant team members of baseline setup (issue comment or messenger)

## 6. Change Management After Baseline

After a baseline is established, even if changes occur in deliverables based on that baseline, the following procedure **must be followed**.

### 6.1 Change Request Procedure

1. **Create change issue**: Register required changes as a GitHub issue
   - Title: Register with `[BUG]` or `[FEATURE]` type
   - Body: Specify reason for change, impact scope, and risk level
2. **Impact analysis**: CM manager and relevant developers analyze the impact scope of the change
3. **Stakeholder approval**:
   - Changes to major baseline targets: Approval from all stakeholders required
   - Changes to minor baseline targets: Approval from relevant team members
4. **Implement change**: Develop in a feature branch and create PR based on approved issue
5. **Review and merge**: Merge after code review and CI pass
6. **Set new baseline** (if needed): Increase patch version and set a new baseline

### 6.2 Prohibited Changes

- Deleting or rewriting (force push) official baseline tags without approval is prohibited
- Changing history of the commit pointed to by a baseline tag is prohibited
- Distributing official baseline deliverables without stakeholder approval is prohibited

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
