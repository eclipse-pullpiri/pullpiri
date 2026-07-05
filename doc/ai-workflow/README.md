<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# AI Workflow Directory Guide

This directory contains AI-facing working materials only.

## Directory Boundary

- `common/`
  - Shared context used by AI runs (coding standards, glossary, quality gates, architecture principles).
- `10-ai-work/core/prompts/`
  - Executable prompts for AI runs.
  - File suffix rule: `*.prompt.md`
- `10-ai-work/core/templates/`
  - AI workflow document templates (contracts/checklists/process forms).
  - File suffix rule: `*.template.md`

## Placement Rules

1. If the file is copied directly into an AI prompt input, place it under `prompts/`.
2. If the file is a reusable document format, place it under `templates/`.
3. Human-reviewed final design artifacts must live under `doc/architecture/`.
4. Canonical design templates must live under `doc/architecture/templates/`.
