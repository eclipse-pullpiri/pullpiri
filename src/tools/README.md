<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# PULLPIRI Tools

These are tools that helps in the development of `Pullpiri`.

## idl2rs

In order to use DDS, you need to use the same IDL files on both pub/sub sides.
This tool makes it easy to convert IDL files to rust `.rs` files.

## ppr

This is a CLI tool that provides access to the Pullpiri apiserver.

## yamlvalidator

In order for Pullpiri to work, it is necessary to generate the correct resource
files in yaml. This tool allows you to determine if a created yaml is valid
inside Pullpiri.
