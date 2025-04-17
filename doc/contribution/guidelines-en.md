# GitHub Development Workflow Guidelines

**Date**: 2025-04-17  
**Author**: edo-lee_LGESDV

## Table of Contents

1. [Issue Registration Rules](#1-issue-registration-rules)
2. [Branch Creation Rules](#2-branch-creation-rules)
3. [Commit Rules](#3-commit-rules)
4. [Labeling Rules by Stage](#4-labeling-rules-by-stage)
5. [Step-by-Step Workflow Guide](#5-step-by-step-workflow-guide)
6. [Automation Setup Guide](#6-automation-setup-guide)

## 1. Issue Registration Rules

### Issue Type Classification

- **FEATURE**: Requirement Issue (Parent Issue)
- **TASK**: Development Task Issue (Child Issue)
- **BUG**: Bug Fix Issue

### Issue Title Format

[Type] Title

Example:

- `[FEATURE] User Authentication System Implementation`
- `[TASK] Login Page UI Development`
- `[BUG] Password Reset Email Sending Failure`

### Issue Body Template

#### Requirement (REQ) Issue Template

```markdown
---
name: Requirement
about: New feature requirement
title: '[FEATURE] '
labels: requirement, status:backlog
assignees: ''
---
```

## 📝 Requirement Description
<!-- Detailed description of the requirement -->

## 📋 Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2

## 📎 Related Documents/References
<!-- Links to related documents -->

## 📌 Subtasks
<!-- Automatically updated -->

## 🧪 Testing Plan

- [ ] Unit Test:
- [ ] Integration Test:
- [ ] Performance Test:

## 📊 Test Results
<!-- Automatically updated after issue closure -->

## Development Task (TASK) Issue Template

```markdown
---
name: Development Task
about: Development task to be implemented
title: '[TASK] '
labels: task, status:todo
assignees: ''
---
```

## 📝 Task Description
<!-- Description of the task to be performed -->

## 📋 Checklist

- [ ] Item 1
- [ ] Item 2

## 🔗 Related Requirement
<!-- Link to parent requirement in "Relates to #issue_number" format -->
Relates to #

## 📐 Implementation Guidelines
<!-- Reference material for implementation -->

## 🧪 Testing Method
<!-- Testing method after implementation -->

## Issue Relationship Setup

```text
Connect Requirement (REQ) and Development Task (TASK): Specify Relates to #requirement_number in the TASK issue description.
Track subtasks in the requirement issue:

## 📌 Subtasks
- [ ] #123 Login Page UI Development
- [ ] #124 Backend Authentication API Implementation
```

## 2. Branch Creation Rules

### Branch Naming Convention

```text
<type>/<issue_number>-<short-description>
```

### Branch Types

- **feat**: New feature development
- **fix**: Bug fix
- **refactor**: Code refactoring
- **docs**: Documentation work
- **test**: Test code work
- **chore**: Other maintenance work

### Examples

- `feat/123-user-authentication`
- `fix/145-password-reset-bug`
- `docs/167-api-documentation`

### Branch Creation Procedure

  1. Use "Development" > "Create a branch" on the issue page, or
  2. From the command line:

```bash
    git checkout -b feat/123-user-login main
```

## 3. Commit Rules

## Commit Message Format

```text
<type>(<scope>): <description> [#issue-number]
```

## Commit Types

- **feat**: New feature
- **fix**: Bug fix
- **docs**: Documentation changes
- **style**: Code formatting, missing semicolons, etc.
- **refactor**: Code refactoring
- **test**: Test-related code
- **chore**: Build tasks, package manager configuration, etc.

## Examples

- `feat(auth): Implement social login [#123]`
- `fix(ui): Fix button overlap on mobile [#145]`
- `docs(api): Update API documentation [#167]`

## Detailed Commit Description (Optional)

```text
<type>(<scope>): <description> [#issue-number]

<Detailed explanation>

<Caveats or Breaking Changes>

<Related Issues (Closes, Fixes, Resolves)>

<Related Issues (Closes, Fixes, Resolves)>
```

## PR Body Format

```markdown
## 📝 PR Description
<!-- Description of the changes -->

## 🔗 Related Issue
<!-- Link to the issue this PR resolves (Use Closes, Fixes, Resolves keywords) -->
Closes #

## 🧪 Test Method
<!-- Description of the test method -->

## 📸 Screenshots
<!-- Attach screenshots if there are UI changes -->

## ✅ Checklist
- [ ] Code conventions are followed
- [ ] Tests are added/modified
- [ ] Documentation is updated (if necessary)
```

## 4. Labeling Rules By Stage

### Label System

#### 1. Status Labels (status:*)

- `status:backlog` - Issue in the backlog
- `status:todo` - Issue in the to-do list
- `status:in-progress` - Issue in progress
- `status:review` -  Under review
- `status:blocked` - Blocked
- `status:done` - Completed

#### 2. Type Labels  (type:*)

- `type:requirement` - Requirement issue
- `type:task` - Development task issue
- `type:bug` - Bug issue
- `type:enhancement` - Feature enhancement
- `type:documentation` - Documentation task

#### 3. Priority Labels (priority:*)

- `priority:critical` - Highest priority
- `priority:high` - High priority
- `priority:medium` - Medium priority
- `priority:low` - Low priority

#### 4. Test Status Labels(test:*)

- `test:pending` - Test pending
- `test:running` - Test running
- `test:passed` - Test passed
- `test:failed` - Test failed

### Label Color Guide

```text
Status labels: Blue shades
Type labels: Green shades
Priority labels: Red/Yellow shades
Complexity labels: Purple shades
Test status labels: Gray/Black shades
```

## 5. Step-by-Step Workflow Guide

### 1. Create Requirement Issue

- Title: [REQ] Requirement Title
- Labels: type:requirement, status:backlog
- Write detailed description

### 2. Create Development Task Issue

- Title: [TASK] Task Title
- Labels: type:task, status:todo
- Link to parent issue: Relates to #requirement_number

### 3. Create Branch and Develop

- Branch name: feat/issue_number-task_name
- Change issue status: status:in-progress

### 4. Commit and Push

- Commit message: feat(scope): Implementation details [#issue_number]

### 5. Create Pull Request

- Title: [Issue Type] Issue Title (#issue_number)
- Include Closes #issue_number in the body
- Label: status:review

### 6. Code Review and Merge

- Assign reviewers
- Merge after approval
- Issue automatically closes

### 7.  Run Tests

- Trigger test execution
- Update labels based on test results: test:passed or test:failed
- Update the requirement issue with test results

## 6. Automation Setup Guide

### Branch Protection Rules

1. Repository > Settings > Branches > Branch protection rules
2. Configure protection rules for the main/master branch:

- Require pull request reviews
- Require status checks to pass
- Require linear history

### Label Automation Workflow

Implement the following automation using GitHub Actions:

- Set initial labels when creating issues/PRs
- Update issue status when creating a branch
- Run tests and update labels when merging a PR

## Workflow Diagram

```text
Create Requirement Issue
       ↓
  Create Sub-tasks
       ↓
  Create Branch
       ↓
    Fork Repo
       ↓
  Development Work
       ↓
  Commit and Push
       ↓
    Create PR
       ↓
  Code Review
       ↓
  Approve and Merge PR
       ↓
  Run Automated Tests
       ↓
  Close Issue and Update Results
```
