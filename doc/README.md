<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Documentation Structure

This directory is organized by audience and purpose.

- `architecture/`: human-reviewed design artifacts and design templates.
	- `en/`, `ko/`: finalized HLD/LLD documents by language.
	- `templates/`: canonical design templates (HLD, LLD, Function Spec, ADR, NFR, contracts, test strategy).
- `guides/`: user/developer operational guides and execution templates.
	- `templates/`: test specs, release/readiness, and change summary templates.
- `contribution/`: contribution policy, workflow, and issue/planning templates.
	- `templates/`: problem statement and PRD scope templates.
- `ai-workflow/`: AI-only working area.
	- `common/`: shared AI context (coding standards, glossary, quality gates, architecture principles).
	- `10-ai-work/`: AI execution assets.
		- `core/prompts/`: executable prompt files (`*.prompt.md`).
		- `core/templates/`: AI workflow document templates (`*.template.md`).
- `resources/`: resource-domain documents (`Package`, `Scenario`).
- `images/`: documentation images and diagrams.
- `scripts/`: helper scripts referenced by docs.

## Operating Rules

1. Canonical design templates must live under `architecture/templates/`.
2. `ai-workflow/` must contain AI-facing materials only.
3. Prompts and templates must be separated by directory and filename suffix (`.prompt.md` vs `.template.md`).
4. Human-facing final documents must not be stored under `ai-workflow/`.
5. If a document appears in multiple places, exactly one canonical source must be declared.
