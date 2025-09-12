/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &[
                "proto/apiserver.proto",
                "proto/actioncontroller.proto",
                "proto/filtergateway.proto",
                "proto/monitoringserver.proto",
                "proto/nodeagent.proto",
                "proto/policymanager.proto",
                "proto/statemanager.proto",
                "proto/pharos_service.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
