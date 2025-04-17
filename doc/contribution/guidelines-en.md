# GitHub Development Workflow Guidelines

**Date**: 2025-04-17
**Author**: edo-lee_LGESDV

## Table of Contents

1. [Issue Registration Rules](#1-issue-registration-rules)
2. [Branch Creation Rules](#2-branch-creation-rules)
3. [Commit Rules](#3-commit-rules)
4. [Stage-by-Stage Labeling Rules](#4-stage-by-stage-labeling-rules)
5. [Workflow Step-by-Step Guide](#5-workflow-step-by-step-guide)
6. [Automation Setup Guide](#6-automation-setup-guide)

## 1. Issue Registration Rules

### Issue Type Classification

- **REQ**: Requirement issue (parent issue)
- **TASK**: Development task issue (child issue)
- **BUG**: Bug fix issue
- **DOCS**: Documentation issue
- **TEST**: Test-related issue

### Issue Title Format

```text
[Type] Title
```

Examples:

- `[REQ] Implement User Authentication System`
- `[TASK] Develop Login Page UI`
- `[BUG] Password Reset Email Sending Failure`

### Issue Body Templates

#### Requirement (REQ) Issue Template

```markdown
---
name: Requirement
about: New feature requirement
title: '[REQ] '
labels: requirement, status:backlog
assignees: ''
---

## 📝 Requirement Description
<!-- Detailed description of the requirement -->

## 📋 Acceptance Criteria
- [ ] Criteria 1
- [ ] Criteria 2

## 📎 Related Documents/References
<!-- Links to related documents -->

## 📌 Sub-tasks
<!-- Automatically updated -->

## 🧪 Test Plan
- [ ] Unit tests:
- [ ] Integration tests:
- [ ] Performance tests:

## 📊 Test Results
<!-- Automatically updated after issue closure -->
```

#### Development Task (TASK) Issue Template

```markdown
---
name: Development Task
about: Development work to be implemented
title: '[TASK] '
labels: task, status:todo
assignees: ''
---

## 📝 Task Description
<!-- Description of the task to be performed -->

## 📋 Checklist
- [ ] Item 1
- [ ] Item 2

## 🔗 Related Requirement
<!-- Connect to parent requirement with "Relates to #issue_number" format -->
Relates to #

## 📐 Implementation Guidelines
<!-- Content to refer to during implementation -->

## 🧪 Test Method
<!-- How to test after implementation -->
```

### Issue Relationship Setup

- Connecting Requirements (REQ) and Tasks (TASK): Specify `Relates to #requirement_number` in the TASK issue description
- Tracking sub-tasks with task lists in requirement issues:

```markdown
## 📌 Sub-tasks
- [ ] #123 Develop Login Page UI
- [ ] #124 Implement Backend Authentication API
```

---

## 2. Branch Creation Rules

### Branch Naming Convention

```text
<type>/<issue-number>-<brief-description>
```

### Branch Types

- **feat**: New feature development
- **fix**: Bug fix
- **refactor**: Code refactoring
- **docs**: Documentation work
- **test**: Test code work
- **chore**: Other maintenance tasks

### Examples

- `feat/123-user-authentication`
- `fix/145-password-reset-bug`
- `docs/167-api-documentation`

### Branch Creation Procedure

1. Use "Development" > "Create a branch" from the issue page, or
2. From the command line:

```bash
git checkout -b feat/123-user-login main
```

## 3. Commit Rules

### Commit Message Format

```text
<type>(<scope>): <description> [#issue-number]
```

### Commit Types

- **feat**:
