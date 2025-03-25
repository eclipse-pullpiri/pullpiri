/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
mod artifact;
mod bluechi;
mod grpc;
mod manager;
mod route;

#[tokio::main]
async fn main() {
    manager::initialize().await
}
