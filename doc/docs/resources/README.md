<!--
SPDX-License-Identifier: Apache-2.0
-->

# Piccolo resources

Piccolo uses resources called `Package` and `Scenario` to manage vehicle services. The package actually represents the vehicle service, and the scenario represents the conditions, policies, etc. under which this service will be executed.

Many services with MSA structure can be easily implemented through packages, and the operations of creating/updating/deleting services according to desired requirements can be controlled through scenarios.

## Scenario

A scenario consists of three components: condition, action, and target.

[Scenario](./scenario.md)

## Package

A package is a folder bundled in tar format that represents a vehicle service.

[Package](./package.md)
