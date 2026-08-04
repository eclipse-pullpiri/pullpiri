<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# PULLPIRI Tools

These are tools that helps in the development of `Pullpiri`.

## idl2rs

In order to use DDS, you need to use the same IDL files on both pub/sub sides.
This tool makes it easy to convert IDL files to rust `.rs` files.

## scaling_client

A command-line client for the Dynamic Resource Scaling feature (#514 / #526)
that applies new CPU / memory limits to a running container at runtime.
See [dynamic-resource-scale/README.md](./dynamic-resource-scale/README.md) for
build and usage instructions.
